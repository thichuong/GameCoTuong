use crate::engine::eval::SimpleEvaluator;
use crate::engine::{Evaluator, Move, Searcher};
use crate::logic::board::{Board, Color};
use crate::logic::game::{GameState, GameStatus};
use crate::logic::rules::is_valid_move;

pub struct AlphaBetaEngine {
    evaluator: SimpleEvaluator,
    nodes_searched: u32,
}

impl AlphaBetaEngine {
    pub fn new() -> Self {
        Self {
            evaluator: SimpleEvaluator,
            nodes_searched: 0,
        }
    }

    fn alpha_beta(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
        mut beta: i32,
        turn: Color,
    ) -> i32 {
        self.nodes_searched += 1;

        if depth == 0 {
            return self.quiescence(board, alpha, beta, turn);
        }

        // TODO: Check for game over (mate/stalemate) - simplified for now

        let moves = self.generate_moves(board, turn);
        if moves.is_empty() {
            // No moves: Checkmate or Stalemate
            // For simplicity, return large negative if checked, 0 if stalemate
            // We need is_in_check here, but let's assume worst case for now
            return -20000 + (10 - depth as i32); // Prefer faster mate
        }

        let mut best_score = -30000;

        for mv in moves {
            let mut next_board = board.clone();
            // Execute move (simplified, assume valid)
            let piece = next_board.grid[mv.from_row][mv.from_col].take().unwrap();
            next_board.grid[mv.to_row][mv.to_col] = Some(piece);

            let score = -self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite());

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

        best_score
    }

    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32, turn: Color) -> i32 {
        self.nodes_searched += 1;

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

        // Generate ONLY captures
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
                        // Brute force all targets (optimization needed later)
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(board, r, c, tr, tc, turn).is_ok() {
                                    // Score move for ordering (Captures first)
                                    let score = if let Some(target) = board.get_piece(tr, tc) {
                                        100 // Capture bonus
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
        // Sort descending by score
        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }

    fn generate_captures(&self, board: &Board, turn: Color) -> Vec<Move> {
        // Similar to generate_moves but only adds if target exists
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
                                            score: 100, // All captures are good candidates
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
    fn search(&mut self, game_state: &GameState, depth: u8) -> Option<Move> {
        self.nodes_searched = 0;
        let board = &game_state.board;
        let turn = game_state.turn;

        // Iterative Deepening (Simplified: just run depth 1..N)
        let mut best_move = None;

        for d in 1..=depth {
            let mut alpha = -30000;
            let beta = 30000;
            let mut current_best_move = None;
            let mut best_score = -30000;

            let moves = self.generate_moves(board, turn);

            for mv in moves {
                let mut next_board = board.clone();
                let piece = next_board.grid[mv.from_row][mv.from_col].take().unwrap();
                next_board.grid[mv.to_row][mv.to_col] = Some(piece);

                let score = -self.alpha_beta(&next_board, d - 1, -beta, -alpha, turn.opposite());

                if score > best_score {
                    best_score = score;
                    current_best_move = Some(mv);
                }
                if score > alpha {
                    alpha = score;
                }
            }

            if let Some(mv) = current_best_move {
                best_move = Some(mv);
                // leptos::logging::log!("Depth {}: Best move {:?} Score {} Nodes {}", d, mv, best_score, self.nodes_searched);
            }
        }

        best_move
    }
}
