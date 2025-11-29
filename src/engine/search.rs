use crate::engine::config::EngineConfig;
use crate::engine::eval::SimpleEvaluator;
use crate::engine::move_list::MoveList;
use crate::engine::zobrist::{TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, Color, PieceType};
use crate::logic::game::GameState;
use crate::logic::rules::is_valid_move;
use std::sync::Arc;
pub struct AlphaBetaEngine {
    config: Arc<EngineConfig>,
    evaluator: SimpleEvaluator,
    tt: TranspositionTable,
    killer_moves: [[Option<Move>; 2]; 64], // Max depth 64
    history_stack: Vec<u64>,
    pub history_table: Box<[[i32; 90]]>,
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
}

impl AlphaBetaEngine {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        Self {
            evaluator: SimpleEvaluator::new(config.clone()),
            config,
            tt: TranspositionTable::new(1), // 1MB (approx 65536 entries)
            killer_moves: [[None; 2]; 64],
            history_stack: Vec::with_capacity(64),
            history_table: vec![[0; 90]; 90].into_boxed_slice(),
            nodes_searched: 0,
            start_time: 0.0,
            time_limit: None,
        }
    }

    fn now() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .expect("should have window")
                .performance()
                .expect("should have performance")
                .now()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap_or_default();
            #[allow(clippy::cast_precision_loss)]
            let time_ms = (since_the_epoch.as_secs() as f64) * 1000.0
                + (f64::from(since_the_epoch.subsec_nanos()) / 1_000_000.0);
            time_ms
        }
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
            // If we timeout, we might have pushed to stack?
            // No, check_time is called at start.
            // But if we recurse, we pushed.
            // If check_time returns true, we return None.
            // The caller (parent alpha_beta) will see None and propagate it.
            // The parent MUST pop the stack if it pushed.
            // My implementation:
            // self.history_stack.push(hash);
            // ...
            // score = self.alpha_beta(...)
            // if score is None -> return None.
            // Wait, if I return None here, I haven't popped!
            // I need to pop if I return None?
            // No, `check_time` is at the very beginning, BEFORE push.
            // So it's fine.
            return None; // Time out
        }

        // Repetition Check
        let hash = board.zobrist_hash;
        if self.history_stack.contains(&hash) {
            return Some(0);
        }
        self.history_stack.push(hash);

        // TT Probe
        if let Some(score) = self.tt.probe(hash, depth, alpha, beta) {
            self.history_stack.pop();
            return Some(score);
        }

        if depth == 0 {
            let score = self.quiescence(board, alpha, beta, turn);
            self.history_stack.pop();
            return Some(score);
        }

        let mut best_move = None;
        if let Some(mv) = self.tt.get_move(hash) {
            best_move = Some(mv);
        }

        let mut moves = self.generate_moves(board, turn, best_move, depth);
        if moves.is_empty() {
            // No moves: Checkmate or Stalemate
            self.history_stack.pop();
            return Some(-20000 + (10 - i32::from(depth)));
        }

        // Pruning: Discard ratio
        if depth >= 3 && self.config.pruning_discard_ratio > 0 {
            let total = moves.len();
            let keep_ratio = 100 - self.config.pruning_discard_ratio;
            let keep_count = (total * keep_ratio as usize) / 100;
            let keep_count = keep_count.max(1); // Always keep at least the best move
            if keep_count < total {
                moves.truncate(keep_count);
            }
        }

        let mut best_score = -30000;
        let mut best_move_this_node = None; // Renamed to avoid conflict with `best_move` from TT
        let alpha_orig = alpha;
        let moves_searched = 0;

        for mv in moves {
            let mut next_board = board.clone();
            next_board.apply_move(&mv, turn);

            let mut score;
            if moves_searched == 0 {
                // First move: Full window search
                let val = self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite());
                match val {
                    None => {
                        self.history_stack.pop();
                        return None;
                    }
                    Some(v) => score = -v,
                }
            } else {
                // Late moves: Null window search (PVS)
                let val =
                    self.alpha_beta(&next_board, depth - 1, -alpha - 1, -alpha, turn.opposite());
                match val {
                    None => {
                        self.history_stack.pop();
                        return None;
                    }
                    Some(v) => score = -v,
                }

                if score > alpha && score < beta {
                    // Fail high in null window, re-search with full window
                    // We need to re-search because the move might be better than alpha
                    // but we only proved it's > alpha, not exact score.
                    // Actually, if score > alpha, we found a better move.
                    // If score >= beta, we cutoff.
                    // If alpha < score < beta, we need exact score.
                    // Re-search:
                    let val =
                        self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite());
                    match val {
                        None => {
                            self.history_stack.pop();
                            return None;
                        }
                        Some(v) => score = -v,
                    }
                    // Use the re-search score if it's valid (it should be)
                    // But wait, if re_score returns None (timeout), we should propagate it.
                    // The ? operator handles None.
                    // So we just update score.
                    // However, we can't assign to `score` easily if it's let binding.
                    // Let's restructure.
                    // But wait, `score` is shadowed? No.
                    // We need to update `score`.
                    // Rust doesn't allow re-assignment if not mut.
                    // Let's make score mutable or handle it.
                    // Actually, simpler:
                    /*
                    score = ...
                    if score > alpha && score < beta {
                        score = ...
                    }
                    */
                    // But `score` is defined inside loop? No, I defined `let score;` above.
                    // Wait, I can't reassign `score` if it's not mut.
                    // Let's make it `let mut score`.
                }
            }
            // Wait, I need to implement the re-search logic correctly.
            // Let's rewrite the loop body.

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(mv);
            }
            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                // Killer Heuristic: Store quiet move that caused cutoff
                // A move is considered quiet if it's not a capture (score < 1_000_000)
                // Store Killer Move
                self.store_killer(depth, mv);
                // History Heuristic
                let from = mv.from_row * 9 + mv.from_col;
                let to = mv.to_row * 9 + mv.to_col;
                if let Some(row) = self.history_table.get_mut(from) {
                    if let Some(score) = row.get_mut(to) {
                        *score += i32::from(depth) * i32::from(depth);
                    }
                }

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
        self.tt
            .store(hash, depth, best_score, flag, best_move_this_node);

        self.history_stack.pop();
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

    fn generate_moves(
        &self,
        board: &Board,
        turn: Color,
        best_move: Option<Move>,
        depth: u8,
    ) -> MoveList {
        let mut moves = MoveList::new();
        let killers = if (depth as usize) < self.killer_moves.len() {
            self.killer_moves.get(depth as usize).unwrap_or(&[None; 2])
        } else {
            &[None; 2]
        };

        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = board.get_piece(r, c) {
                    if p.color == turn {
                        for tr in 0..10 {
                            for tc in 0..9 {
                                if is_valid_move(board, r, c, tr, tc, turn).is_ok() {
                                    let mut score;

                                    // Check if this is the hash move
                                    let is_hash_move = if let Some(bm) = best_move {
                                        bm.from_row == r
                                            && bm.from_col == c
                                            && bm.to_row == tr
                                            && bm.to_col == tc
                                    } else {
                                        false
                                    };

                                    let is_killer_move = killers.iter().any(|k| {
                                        if let Some(km) = k {
                                            km.from_row == r
                                                && km.from_col == c
                                                && km.to_row == tr
                                                && km.to_col == tc
                                        } else {
                                            false
                                        }
                                    });

                                    if is_hash_move {
                                        score = self.config.score_hash_move;
                                    } else if let Some(target) = board.get_piece(tr, tc) {
                                        // MVV-LVA
                                        let victim_val = self.get_piece_value(target.piece_type);
                                        let attacker_val = self.get_piece_value(p.piece_type);
                                        score = self.config.score_capture_base + victim_val
                                            - (attacker_val / 10);
                                    } else if is_killer_move {
                                        score = self.config.score_killer_move;
                                    } else {
                                        // History Heuristic
                                        let from = r * 9 + c;
                                        let to = tr * 9 + tc;
                                        #[allow(clippy::indexing_slicing)]
                                        {
                                            score = self.history_table[from][to];
                                        }
                                        if score > self.config.score_history_max {
                                            score = self.config.score_history_max;
                                        }
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

    fn generate_captures(&self, board: &Board, turn: Color) -> MoveList {
        let mut moves = MoveList::new();
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
                                        let victim_val = self.get_piece_value(target.piece_type);
                                        let attacker_val = self.get_piece_value(p.piece_type);
                                        let score = self.config.score_capture_base + victim_val
                                            - (attacker_val / 10);

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

    fn store_killer(&mut self, depth: u8, mv: Move) {
        if depth >= 64 {
            return;
        }
        let d = depth as usize;
        // Shift: 0 -> 1, New -> 0
        if let Some(killers) = self.killer_moves.get_mut(d) {
            if killers[0] != Some(mv) {
                killers[1] = killers[0];
                killers[0] = Some(mv);
            }
        }
    }

    fn get_piece_value(&self, pt: PieceType) -> i32 {
        match pt {
            PieceType::General => self.config.val_king,
            PieceType::Chariot => self.config.val_rook,
            PieceType::Cannon => self.config.val_cannon,
            PieceType::Horse => self.config.val_horse,
            PieceType::Elephant => self.config.val_elephant,
            PieceType::Advisor => self.config.val_advisor,
            PieceType::Soldier => self.config.val_pawn,
        }
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
        self.history_stack.clear();

        let (max_depth, time_limit) = match limit {
            SearchLimit::Depth(d) => (d.min(63), None),
            #[allow(clippy::cast_precision_loss)]
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

            let moves = self.generate_moves(board, turn, tt_move, d);

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
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    time_ms: elapsed as u64,
                },
            )
        })
    }
}
