use crate::engine::eval::SimpleEvaluator;
use crate::engine::{Evaluator, Move, SearchLimit, Searcher};
use crate::logic::board::{Board, Color};
use crate::logic::game::GameState;
use crate::logic::rules::is_valid_move;
pub struct AlphaBetaEngine {
    evaluator: SimpleEvaluator,
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
}

impl AlphaBetaEngine {
    pub fn new() -> Self {
        Self {
            evaluator: SimpleEvaluator,
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

        if depth == 0 {
            return Some(self.quiescence(board, alpha, beta, turn));
        }

        let moves = self.generate_moves(board, turn);
        if moves.is_empty() {
            // No moves: Checkmate or Stalemate
            return Some(-20000 + (10 - depth as i32));
        }

        let mut best_score = -30000;

        for mv in moves {
            let mut next_board = board.clone();
            let piece = next_board.grid[mv.from_row][mv.from_col].take().unwrap();
            next_board.grid[mv.to_row][mv.to_col] = Some(piece);

            let score = -self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite())?;

            if score > best_score {
                best_score = score;
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                break; // Beta cutoff
            }
        }

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
            let piece = next_board.grid[mv.from_row][mv.from_col].take().unwrap();
            next_board.grid[mv.to_row][mv.to_col] = Some(piece);

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

    fn generate_moves(&self, board: &Board, turn: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = board.get_piece(r, c) {
                    if p.color == turn {
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(board, r, c, tr, tc, turn).is_ok() {
                                    let score = if let Some(_target) = board.get_piece(tr, tc) {
                                        100
                                    } else {
                                        0
                                    };
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
                                        moves.push(Move {
                                            from_row: r,
                                            from_col: c,
                                            to_row: tr,
                                            to_col: tc,
                                            score: 100,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        moves
    }
}

impl Searcher for AlphaBetaEngine {
    fn search(&mut self, game_state: &GameState, limit: SearchLimit) -> Option<Move> {
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

        for d in 1..=max_depth {
            let mut alpha = -30000;
            let beta = 30000;
            let mut current_best_move = None;
            let mut best_score = -30000;

            let moves = self.generate_moves(board, turn);

            // Check time before starting a new depth
            if self.check_time() {
                break;
            }

            let mut time_out = false;

            for mv in moves {
                let mut next_board = board.clone();
                let piece = next_board.grid[mv.from_row][mv.from_col].take().unwrap();
                next_board.grid[mv.to_row][mv.to_col] = Some(piece);

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
                }
                break;
            } else if let Some(mv) = current_best_move {
                best_move = Some(mv);
                // leptos::logging::log!("Depth {}: Best move {:?} Score {} Nodes {}", d, mv, best_score, self.nodes_searched);
            }
        }

        best_move
    }
}
