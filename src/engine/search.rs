use crate::engine::eval::SimpleEvaluator;
use crate::engine::eval_constants::*;
use crate::engine::zobrist::{TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, Color, PieceType};
use crate::logic::game::GameState;
use crate::logic::rules::is_valid_move;
pub struct AlphaBetaEngine {
    evaluator: SimpleEvaluator,
    tt: TranspositionTable,
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
}

impl AlphaBetaEngine {
    pub fn new() -> Self {
        Self {
            evaluator: SimpleEvaluator,
            tt: TranspositionTable::new(1), // 1MB (approx 65536 entries)
            nodes_searched: 0,
            start_time: 0.0,
            time_limit: None,
        }
    }

    fn now() -> f64 {
        web_sys::window()
            .expect("should have window")
            .performance()
            .expect("should have performance")
            .now()
    }

    #[allow(clippy::manual_is_multiple_of)]
    fn check_time(&self) -> bool {
        if let Some(limit) = self.time_limit {
            if self.nodes_searched % 1024 == 0 {
                let elapsed = Self::now() - self.start_time;
                if elapsed > limit {
                    return true;
                }
            }
        }
        false
    }

    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        turn: Color,
    ) -> Option<i32> {
        self.nodes_searched += 1;

        if self.check_time() {
            return None; // Time out
        }

        // TT Probe
        let hash = board.zobrist_hash;
        if let Some(score) = self.tt.probe(hash, depth, alpha, beta) {
            return Some(score);
        }

        if depth == 0 {
            return Some(self.quiescence(board, alpha, beta, turn));
        }

        let mut best_move = None;
        if let Some(mv) = self.tt.get_move(hash) {
            best_move = Some(mv);
        }

        let moves = self.generate_moves(board, turn, best_move);
        if moves.is_empty() {
            // No moves: Checkmate or Stalemate
            return Some(-20000 + (10 - depth as i32));
        }

        let mut best_score = -30000;
        let mut best_move = None;
        let alpha_orig = alpha;

        for mv in moves {
            let mut next_board = board.clone();
            next_board.apply_move(&mv, turn);

            let score = -self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite())?;

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                break; // Beta cutoff
            }
        }

        // TT Store
        let flag = if best_score <= alpha_orig {
            TTFlag::UpperBound
        } else if best_score >= beta {
            TTFlag::LowerBound
        } else {
            TTFlag::Exact
        };
        self.tt.store(hash, depth, best_score, flag, best_move);

        Some(best_score)
    }

    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32, turn: Color) -> i32 {
        self.nodes_searched += 1;

        // Q-Search doesn't check time strictly to avoid partial evaluations,
        // but we could add it if needed. For now, let it finish.

        let stand_pat = if turn == Color::Red {
            self.evaluator.evaluate(board)
        } else {
            -self.evaluator.evaluate(board)
        };

        if stand_pat >= beta {
            return beta;
        }
        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let captures = self.generate_captures(board, turn);

        for mv in captures {
            let mut next_board = board.clone();
            next_board.apply_move(&mv, turn);

            let score = -self.quiescence(&next_board, -beta, -alpha, turn.opposite());

            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    fn generate_moves(&self, board: &Board, turn: Color, best_move: Option<Move>) -> Vec<Move> {
        let mut moves = Vec::new();
        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = board.get_piece(r, c) {
                    if p.color == turn {
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(board, r, c, tr, tc, turn).is_ok() {
                                    let mut score = 0;

                                    // Check if this is the hash move
                                    let is_hash_move = if let Some(bm) = best_move {
                                        bm.from_row == r
                                            && bm.from_col == c
                                            && bm.to_row == tr
                                            && bm.to_col == tc
                                    } else {
                                        false
                                    };

                                    if is_hash_move {
                                        score = 20000;
                                    } else if let Some(target) = board.get_piece(tr, tc) {
                                        // MVV-LVA
                                        // Victim value - Attacker value (small offset)
                                        // We want to prioritize capturing high value pieces with low value pieces.
                                        let victim_val = get_piece_value(target.piece_type);
                                        let attacker_val = get_piece_value(p.piece_type);
                                        score = 1000 + victim_val - (attacker_val / 10);
                                    }

                                    moves.push(Move {
                                        from_row: r,
                                        from_col: c,
                                        to_row: tr,
                                        to_col: tc,
                                        score,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }

    fn generate_captures(&self, board: &Board, turn: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = board.get_piece(r, c) {
                    if p.color == turn {
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if let Some(target) = board.get_piece(tr, tc) {
                                    if target.color != turn
                                        && is_valid_move(board, r, c, tr, tc, turn).is_ok()
                                    {
                                        let victim_val = get_piece_value(target.piece_type);
                                        let attacker_val = get_piece_value(p.piece_type);
                                        let score = 1000 + victim_val - (attacker_val / 10);

                                        moves.push(Move {
                                            from_row: r,
                                            from_col: c,
                                            to_row: tr,
                                            to_col: tc,
                                            score,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }
}

fn get_piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::General => VAL_KING,
        PieceType::Chariot => VAL_ROOK,
        PieceType::Cannon => VAL_CANNON,
        PieceType::Horse => VAL_HORSE,
        PieceType::Elephant => VAL_ELEPHANT,
        PieceType::Advisor => VAL_ADVISOR,
        PieceType::Soldier => VAL_PAWN,
    }
}

impl Searcher for AlphaBetaEngine {
    fn search(
        &mut self,
        game_state: &GameState,
        limit: SearchLimit,
    ) -> Option<(Move, SearchStats)> {
        self.nodes_searched = 0;
        self.start_time = Self::now();

        let (max_depth, time_limit) = match limit {
            SearchLimit::Depth(d) => (d, None),
            SearchLimit::Time(t) => (20, Some(t as f64)), // Max depth 20 for time limit
        };
        self.time_limit = time_limit;

        let board = &game_state.board;
        let turn = game_state.turn;

        let mut best_move = None;
        let mut final_depth = 0;

        for d in 1..=max_depth {
            let mut alpha = -30000;
            let beta = 30000;
            let mut current_best_move = None;
            let mut best_score = -30000;

            // Try to get best move from TT for this depth (or previous)
            let hash = board.zobrist_hash;
            let tt_move = self.tt.get_move(hash);

            let moves = self.generate_moves(board, turn, tt_move);

            // Check time before starting a new depth
            if self.check_time() {
                break;
            }

            let mut time_out = false;

            for mv in moves {
                let mut next_board = board.clone();
                next_board.apply_move(&mv, turn);

                if let Some(score) =
                    self.alpha_beta(&next_board, d - 1, -beta, -alpha, turn.opposite())
                {
                    let score = -score;
                    if score > best_score {
                        best_score = score;
                        current_best_move = Some(mv);
                    }
                    if score > alpha {
                        alpha = score;
                    }
                } else {
                    time_out = true;
                    break;
                }
            }

            if time_out {
                // If we timed out during a depth, don't use partial results unless we have nothing else
                if best_move.is_none() && current_best_move.is_some() {
                    best_move = current_best_move;
                    final_depth = d;
                }
                break;
            } else if let Some(mv) = current_best_move {
                best_move = Some(mv);
                final_depth = d;
            }
        }

        let elapsed = Self::now() - self.start_time;
        best_move.map(|mv| {
            (
                mv,
                SearchStats {
                    depth: final_depth,
                    nodes: self.nodes_searched,
                    time_ms: elapsed as u64,
                },
            )
        })
    }
}
