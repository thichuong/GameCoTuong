use crate::engine::config::EngineConfig;
use crate::engine::eval::SimpleEvaluator;
use crate::engine::move_list::MoveList;
use crate::engine::tt::{TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};
use crate::logic::game::GameState;
use crate::logic::rules::{is_flying_general, is_in_check};
use std::sync::Arc;

pub struct AlphaBetaEngine {
    config: Arc<EngineConfig>,
    evaluator: SimpleEvaluator,
    tt: TranspositionTable,
    killer_moves: [[Option<Move>; 2]; 64],
    history_stack: Vec<u64>,
    pub history_table: Box<[[i32; 90]]>,
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
    dynamic_limits: [usize; 64],
    lmr_table: [[u8; 64]; 64],
}

impl AlphaBetaEngine {
    pub fn new(config: Arc<EngineConfig>) -> Self {
        let dynamic_limits = Self::precompute_limits(&config);
        Self {
            evaluator: SimpleEvaluator::new(config.clone()),
            config,
            tt: TranspositionTable::new(64),
            killer_moves: [[None; 2]; 64],
            history_stack: Vec::with_capacity(64),
            history_table: vec![[0; 90]; 90].into_boxed_slice(),
            nodes_searched: 0,
            start_time: 0.0,
            time_limit: None,
            dynamic_limits,
            lmr_table: Self::precompute_lmr(),
        }
    }

    pub fn update_config(&mut self, config: Arc<EngineConfig>) {
        if config.tt_size_mb != self.config.tt_size_mb {
            self.tt = TranspositionTable::new(config.tt_size_mb);
        }
        self.dynamic_limits = Self::precompute_limits(&config);
        self.evaluator = SimpleEvaluator::new(config.clone());
        self.config = config;
    }

    fn precompute_lmr() -> [[u8; 64]; 64] {
        let mut table = [[0; 64]; 64];
        for (depth, row) in table.iter_mut().enumerate() {
            for (moves_searched, val) in row.iter_mut().enumerate() {
                if depth >= 3 && moves_searched >= 4 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        let r = 1.0
                            + (f64::from(depth as u32).ln()
                                * f64::from(moves_searched as u32).ln())
                                / 1.5; // Increased aggression from 2.0
                        *val = (r as u8).min((depth - 1) as u8);
                    }
                } else {
                    *val = 0;
                }
            }
        }
        table
    }

    fn precompute_limits(config: &EngineConfig) -> [usize; 64] {
        let mut limits = [0; 64];
        for (d, limit) in limits.iter_mut().enumerate() {
            let depth = d as f32;
            let multiplier = config.pruning_multiplier;
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            {
                *limit = (depth * depth).mul_add(multiplier, 8.0) as usize;
            }
        }
        limits
    }

    fn now() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(window) = web_sys::window() {
                return window.performance().map(|p| p.now()).unwrap_or(0.0);
            }
            let global = js_sys::global();
            if let Ok(worker) = global.dyn_into::<web_sys::WorkerGlobalScope>() {
                return worker.performance().map(|p| p.now()).unwrap_or(0.0);
            }
            0.0 // Fail safe instead of panic
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap_or_default();
            #[allow(clippy::cast_precision_loss)]
            let time_ms = (since_the_epoch.as_secs() as f64).mul_add(
                1000.0,
                f64::from(since_the_epoch.subsec_nanos()) / 1_000_000.0,
            );
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

    #[allow(clippy::too_many_lines)]
    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: i32,
        mut beta: i32,
        depth: u8,
        turn: Color,
        excluded_move: Option<Move>,
    ) -> Option<i32> {
        self.nodes_searched += 1;

        if self.check_time() {
            return None;
        }

        // Repetition Check
        // Repetition Check (3-fold)
        let hash = board.zobrist_hash;
        let rep_count = self.history_stack.iter().filter(|&&h| h == hash).count();
        if rep_count >= 2 {
            return Some(0);
        }
        self.history_stack.push(hash);

        // TT Probe
        let tt_entry = self.tt.probe(hash);
        if let Some(entry) = tt_entry {
            if entry.depth >= depth {
                match entry.flag {
                    TTFlag::Exact => {
                        self.history_stack.pop();
                        return Some(entry.score);
                    }
                    TTFlag::LowerBound => {
                        if entry.score >= beta {
                            self.history_stack.pop();
                            return Some(entry.score);
                        }
                        alpha = alpha.max(entry.score);
                    }
                    TTFlag::UpperBound => {
                        if entry.score <= alpha {
                            self.history_stack.pop();
                            return Some(entry.score);
                        }
                        beta = beta.min(entry.score);
                    }
                }
                if alpha >= beta {
                    self.history_stack.pop();
                    return Some(entry.score);
                }
            }
        }

        if depth == 0 {
            let score = self.quiescence(board, alpha, beta, turn);
            self.history_stack.pop();
            return Some(score);
        }

        // ProbCut
        if depth >= self.config.probcut_depth && beta.abs() < 15000 {
            let margin = self.config.probcut_margin;
            let reduction = self.config.probcut_reduction;

            if depth > reduction {
                if let Some(score) = self.alpha_beta(
                    board,
                    -beta - margin,
                    -beta - margin + 1,
                    depth - reduction,
                    turn.opposite(),
                    None,
                ) {
                    if -score >= beta + margin {
                        self.history_stack.pop();
                        return Some(beta);
                    }
                } else {
                    self.history_stack.pop();
                    return None;
                }
            }
        }

        let in_check = is_in_check(board, turn);

        // Reverse Futility Pruning
        if depth <= 3 && !in_check && beta.abs() < 15000 {
            let eval = if turn == Color::Red {
                self.evaluator.evaluate(board)
            } else {
                -self.evaluator.evaluate(board)
            };
            let margin = 120 * i32::from(depth);
            if eval - margin >= beta {
                self.history_stack.pop();
                return Some(eval);
            }
        }

        // Null Move Pruning
        if depth >= 3 && beta.abs() < 15000 && !crate::logic::rules::is_in_check(board, turn) {
            let r = if depth > 6 { 3 } else { 2 }; // Adaptive reduction
                                                   // Null move: switch turn, update hash.
                                                   // Board::apply_null_move toggles side key.
            board.apply_null_move();

            if let Some(score) = self.alpha_beta(
                board,
                -beta,
                -beta + 1,
                depth - 1 - r,
                turn.opposite(),
                None,
            ) {
                board.apply_null_move(); // Undo null move (toggle back)
                if -score >= beta {
                    self.history_stack.pop();
                    return Some(beta);
                }
            } else {
                board.apply_null_move(); // Undo null move
                self.history_stack.pop();
                return None;
            }
        }

        let mut best_move_tt = tt_entry.and_then(|e| e.best_move);

        // Internal Iterative Deepening (IID)
        if best_move_tt.is_none() && depth >= 4 {
            // Search with reduced depth to populate TT
            let _ = self.alpha_beta(board, alpha, beta, depth - 2, turn, None);
            // We don't use the score, just hope TT is populated
            if let Some(entry) = self.tt.probe(hash) {
                best_move_tt = entry.best_move;
            }
        }

        // Singular Extension
        let mut singular_extension = 0;
        // Only trigger SE if:
        // 1. We are at sufficient depth
        // 2. We have a TT move (which is likely the singular move)
        // 3. The TT entry is strong enough (Exact or LowerBound) and has sufficient depth
        // 4. We are NOT already in a singular search (excluded_move is None)
        if depth >= self.config.singular_extension_min_depth
            && excluded_move.is_none()
            && best_move_tt.is_some()
        {
            if let Some(entry) = tt_entry {
                if entry.depth >= depth - 3
                    && (entry.flag == TTFlag::Exact || entry.flag == TTFlag::LowerBound)
                {
                    let margin = self.config.singular_extension_margin;
                    let singular_beta = entry.score - margin;

                    // Search with reduced depth to see if any OTHER move can beat (score - margin)
                    // If all other moves fail low (return < singular_beta), then the TT move is singular.
                    // We exclude the TT move from this search.
                    let val = self.alpha_beta(
                        board,
                        singular_beta - 1,
                        singular_beta,
                        (depth - 1) / 2,
                        turn,
                        best_move_tt,
                    );

                    if let Some(score) = val {
                        if score < singular_beta {
                            singular_extension = 1;
                        }
                    }
                }
            }
        }

        let mut moves = self.generate_moves(board, turn, best_move_tt, depth);

        if in_check {
            // If in check, filter for legal moves immediately.
            // We cannot prune because the only legal moves might be "bad" ones.
            moves.retain(|mv| {
                let captured = board.get_piece(unsafe { BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize) });
                board.apply_move(mv, turn);
                let legal = !is_in_check(board, turn) && !is_flying_general(board);
                board.undo_move(mv, captured, turn);
                legal
            });

            if moves.is_empty() {
                self.history_stack.pop();
                return Some(-self.config.mate_score + (10 - i32::from(depth)));
            }
        }

        if moves.is_empty() {
            self.history_stack.pop();
            return Some(-self.config.mate_score + (10 - i32::from(depth)));
        }

        // Dynamic Limiting Limit Calculation (Moved here, but applied inside loop)
        let dynamic_limit =
            if !in_check && (self.config.pruning_method == 0 || self.config.pruning_method == 2) {
                if (depth as usize) < 64 {
                    self.dynamic_limits[depth as usize]
                } else {
                    moves.len()
                }
            } else {
                moves.len()
            };

        let mut best_score = -200000;
        let mut best_move_this_node = None;
        let mut tt_flag = TTFlag::UpperBound;

        let mut legal_moves_count = 0;
        let mut has_repetition_move = false;

        // Static Eval for Futility Pruning
        let static_eval = if depth <= 3 && !in_check {
            if turn == Color::Red {
                self.evaluator.evaluate(board)
            } else {
                -self.evaluator.evaluate(board)
            }
        } else {
            -200000 // Dummy
        };

        for (moves_searched, mv) in moves.into_iter().enumerate() {
            let is_capture = board
                .get_piece(unsafe {
                    BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
                })
                .is_some();

            // Safe Dynamic Limiting:
            // Only prune if:
            // 1. We have searched enough moves (moves_searched >= dynamic_limit)
            // 2. It is NOT a capture (captures are important)
            // 3. Not in check (already handled by limit calculation usually, but good to be safe)
            if !in_check && moves_searched >= dynamic_limit && !is_capture {
                continue;
            }

            if let Some(ex_mv) = excluded_move {
                if mv == ex_mv {
                    continue;
                }
            }

            // Futility Pruning
            // Prune quiet moves at low depth if static eval is far below alpha
            if !in_check
                && depth <= 3
                && !is_capture
                && (self.config.pruning_method == 0 || self.config.pruning_method == 2)
            {
                let margin = 150 * i32::from(depth);
                if static_eval + margin < alpha {
                    continue;
                }
            }

            let captured = board.get_piece(unsafe {
                BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
            });
            board.apply_move(&mv, turn);

            // Deferred Legality Check
            if !in_check && (is_in_check(board, turn) || is_flying_general(board)) {
                board.undo_move(&mv, captured, turn);
                continue;
            }

            // Absolute Checkmate Detection - REMOVED for performance
            // The search will naturally find checkmates.
            // Explicitly checking for mate in 1 here is too expensive (full move gen for opponent).

            // Repetition Check (Pruning)
            // Check if this position has occurred 2 times before (so this is the 3rd)
            let mut rep_count = 0;
            for &h in &self.history_stack {
                if h == board.zobrist_hash {
                    rep_count += 1;
                }
            }

            if rep_count >= 2 {
                board.undo_move(&mv, captured, turn);
                has_repetition_move = true;
                continue;
            }

            legal_moves_count += 1;

            let score;

            // LMR
            let mut reduction = 0;
            if depth >= 3
                && moves_searched >= 4
                && (self.config.pruning_method == 1 || self.config.pruning_method == 2)
                && !in_check
                && !is_capture
            {
                let d = (depth as usize).min(63);
                let m = moves_searched.min(63);
                reduction = self.lmr_table[d][m];
            }

            let extension = if in_check { 1 } else { 0 } + singular_extension;

            // PVS (Principal Variation Search)
            if moves_searched == 0 {
                // Full window for the first move (PV-node)
                let val = self.alpha_beta(
                    board,
                    -beta,
                    -alpha,
                    depth - 1 + extension,
                    turn.opposite(),
                    None,
                );

                match val {
                    None => {
                        board.undo_move(&mv, captured, turn);
                        self.history_stack.pop();
                        return None;
                    }
                    Some(v) => score = -v,
                }
            } else {
                // Null window search for other moves (Cut-nodes)
                // Try to prove that this move is NOT better than alpha
                let search_depth = depth - 1 - reduction + extension;

                let mut val = self.alpha_beta(
                    board,
                    -alpha - 1,
                    -alpha,
                    search_depth,
                    turn.opposite(),
                    None,
                );

                if let Some(v) = val {
                    let s = -v;
                    if s > alpha {
                        // Fail High: Re-search with full window
                        if reduction > 0 {
                            val = self.alpha_beta(
                                board,
                                -alpha - 1,
                                -alpha,
                                depth - 1 + extension,
                                turn.opposite(),
                                None,
                            );
                        }

                        if let Some(v2) = val {
                            let s2 = -v2;
                            if s2 > alpha {
                                val = self.alpha_beta(
                                    board,
                                    -beta,
                                    -alpha,
                                    depth - 1 + extension,
                                    turn.opposite(),
                                    None,
                                );
                            }
                        }
                    }
                }

                // Handle result
                match val {
                    None => {
                        board.undo_move(&mv, captured, turn);
                        self.history_stack.pop();
                        return None;
                    }
                    Some(v) => score = -v,
                }
            }

            board.undo_move(&mv, captured, turn);

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(mv);
            }
            if score > alpha {
                alpha = score;
                tt_flag = TTFlag::Exact;
            }

            if alpha >= beta {
                self.store_killer(depth, mv);
                let from = (mv.from_row as usize) * 9 + (mv.from_col as usize);
                let to = (mv.to_row as usize) * 9 + (mv.to_col as usize);
                if let Some(row) = self.history_table.get_mut(from) {
                    if let Some(s) = row.get_mut(to) {
                        *s += i32::from(depth) * i32::from(depth);
                    }
                }
                tt_flag = TTFlag::LowerBound;
                break;
            }
        }

        if legal_moves_count == 0 {
            self.history_stack.pop();
            // Checkmate or Stalemate
            if in_check {
                // Checkmate
                return Some(-self.config.mate_score + (10 - i32::from(depth)));
            }
            if has_repetition_move {
                // All legal moves were pruned due to repetition -> Draw
                return Some(0);
            }
            // Stalemate (Loss in Xiangqi)
            return Some(-self.config.mate_score + (10 - i32::from(depth)));
        }

        self.tt
            .store(hash, best_move_this_node, best_score, depth, tt_flag);

        self.history_stack.pop();
        Some(best_score)
    }

    fn quiescence(&mut self, board: &mut Board, mut alpha: i32, beta: i32, turn: Color) -> i32 {
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
            let captured = board.get_piece(unsafe { BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize) });

            // Delta Pruning
            // If stand_pat + capture_value + margin < alpha, we can skip this capture.
            // We need to know the value of the captured piece.
            if let Some(cap_piece) = captured {
                let cap_val = self.get_piece_value(cap_piece.piece_type);
                // Margin of 200 for safety (e.g. positional gains)
                if stand_pat + cap_val + 200 < alpha {
                    continue;
                }
            }

            board.apply_move(&mv, turn);

            // Legality Check (Crucial for Q-Search to avoid illegal captures)
            if crate::logic::rules::is_in_check(board, turn)
                || crate::logic::rules::is_flying_general(board)
            {
                board.undo_move(&mv, captured, turn);
                continue;
            }

            let score = -self.quiescence(board, -beta, -alpha, turn.opposite());

            board.undo_move(&mv, captured, turn);

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
        board: &mut Board,
        turn: Color,
        best_move: Option<Move>,
        depth: u8,
    ) -> MoveList {
        self.generate_moves_internal(board, turn, best_move, depth, false)
    }

    fn generate_captures(&self, board: &mut Board, turn: Color) -> MoveList {
        self.generate_moves_internal(board, turn, None, 0, true)
    }

    fn generate_moves_internal(
        &self,
        board: &mut Board,
        turn: Color,
        best_move: Option<Move>,
        depth: u8,
        only_captures: bool,
    ) -> MoveList {
        let mut moves = MoveList::new();
        let killers = if (depth as usize) < self.killer_moves.len() {
            self.killer_moves.get(depth as usize).unwrap_or(&[None; 2])
        } else {
            &[None; 2]
        };

        use crate::logic::board::BitboardIterator;

        let start = turn.index() * 7;
        for pt_idx in 0..7 {
            let bb = board.bitboards[start + pt_idx];
            for sq in BitboardIterator::new(bb) {
                let (r, c) = Board::index_to_coord(sq);
                let piece_type = match pt_idx {
                    0 => PieceType::General,
                    1 => PieceType::Advisor,
                    2 => PieceType::Elephant,
                    3 => PieceType::Horse,
                    4 => PieceType::Chariot,
                    5 => PieceType::Cannon,
                    6 => PieceType::Soldier,
                    _ => unreachable!(),
                };

                match piece_type {
                    PieceType::Chariot => self.gen_rook_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::Cannon => self.gen_cannon_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::Horse => self.gen_horse_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::Elephant => self.gen_elephant_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::Advisor => self.gen_advisor_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::General => self.gen_king_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                    PieceType::Soldier => self.gen_pawn_moves(
                        board,
                        turn,
                        r,
                        c,
                        &mut moves,
                        best_move,
                        killers,
                        only_captures,
                    ),
                }
            }
        }

        // Apply depth discount - deeper depths get lower scores (relative to root)
        // Formula: score = score * (100 + discount * depth) / 100
        // This scales scores proportionally based on depth.
        let discount_per_depth = self.config.depth_discount;
        if discount_per_depth > 0 {
            let depth_factor = i32::from(depth);
            let scale = 100 + discount_per_depth * depth_factor;
            for mv in moves.iter_mut() {
                // Hash moves should not be discounted as they are already ordered
                if mv.score < self.config.score_hash_move {
                    mv.score = (mv.score * scale) / 100;
                }
            }
        }

        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }

    // is_mate removed as it is too expensive and not needed for search correctness

    #[allow(clippy::too_many_arguments)]
    fn add_move(
        &self,
        board: &mut Board,
        turn: Color,
        from: (usize, usize),
        to: (usize, usize),
        moves: &mut MoveList,
        best_move: Option<Move>,
        killers: &[Option<Move>; 2],
        only_captures: bool,
    ) {
        let (r, c) = from;
        let (tr, tc) = to;

        let target_sq = Board::square_index(tr, tc);
        let is_occupied = (board.occupied & (1 << target_sq)) != 0;

        if is_occupied {
            if (board.get_color_bb(turn) & (1 << target_sq)) != 0 {
                return; // Blocked by friendly
            }
        } else if only_captures {
            return;
        }

        let target = if is_occupied {
            board.get_piece(unsafe { BoardCoordinate::new_unchecked(tr, tc) })
        } else {
            None
        };

        // Legality Check (Self-check & Flying General)
        // Optimization: DEFERRED to search loop.
        // We generate pseudo-legal moves here.

        // Scoring
        let mut score;
        let is_hash_move = best_move.is_some_and(|bm| {
            bm.from_row as usize == r
                && bm.from_col as usize == c
                && bm.to_row as usize == tr
                && bm.to_col as usize == tc
        });

        if is_hash_move {
            score = self.config.score_hash_move;
        } else if let Some(t) = target {
            // MVV-LVA
            let victim_val = self.get_piece_value(t.piece_type);
            let attacker_val = board
                .get_piece(unsafe { BoardCoordinate::new_unchecked(r, c) })
                .map_or(0, |p| self.get_piece_value(p.piece_type));
            score = self.config.score_capture_base + victim_val - (attacker_val / 10);
        } else {
            let is_killer_move = killers.iter().any(|k| {
                k.is_some_and(|km| {
                    km.from_row as usize == r
                        && km.from_col as usize == c
                        && km.to_row as usize == tr
                        && km.to_col as usize == tc
                })
            });

            if is_killer_move {
                score = self.config.score_killer_move;
            } else {
                // History
                let from_idx = r * 9 + c;
                let to_idx = tr * 9 + tc;
                score = *self
                    .history_table
                    .get(from_idx)
                    .and_then(|row| row.get(to_idx))
                    .unwrap_or(&0);
                if score > self.config.score_history_max {
                    score = self.config.score_history_max;
                }
            }
        }

        moves.push(Move {
            from_row: r as u8,
            from_col: c as u8,
            to_row: tr as u8,
            to_col: tc as u8,
            score,
        });
    }

    fn offset(base: usize, delta: i32) -> Option<usize> {
        let base_i = i32::try_from(base).ok()?;
        let res = base_i.checked_add(delta)?;
        usize::try_from(res).ok()
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_rook_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        use crate::logic::lookup::AttackTables;
        let tables = AttackTables::get();

        // Rank attacks (Horizontal)
        let rank_occ = board.occupied_rows[r];
        let rank_attacks = tables.get_rook_attacks(c, rank_occ, 9);

        let mut attacks = rank_attacks;
        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(board, turn, (r, c), (r, col), moves, bm, k, oc);
        }

        // File attacks (Vertical)
        let file_occ = board.occupied_cols[c];
        let file_attacks = tables.get_rook_attacks(r, file_occ, 10);

        let mut attacks = file_attacks;
        while attacks != 0 {
            let row = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(board, turn, (r, c), (row, c), moves, bm, k, oc);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_cannon_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        use crate::logic::lookup::AttackTables;
        let tables = AttackTables::get();

        // Rank attacks (Horizontal)
        let rank_occ = board.occupied_rows[r];
        let rank_attacks = tables.get_cannon_attacks(c, rank_occ, 9);

        let mut attacks = rank_attacks;
        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(board, turn, (r, c), (r, col), moves, bm, k, oc);
        }

        // File attacks (Vertical)
        let file_occ = board.occupied_cols[c];
        let file_attacks = tables.get_cannon_attacks(r, file_occ, 10);

        let mut attacks = file_attacks;
        while attacks != 0 {
            let row = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(board, turn, (r, c), (row, c), moves, bm, k, oc);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_horse_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let moves_offsets = [
            (-2, -1),
            (-2, 1),
            (2, -1),
            (2, 1),
            (-1, -2),
            (-1, 2),
            (1, -2),
            (1, 2),
        ];

        for (dr, dc) in moves_offsets {
            let Some(tr) = Self::offset(r, dr) else {
                continue;
            };
            let Some(tc) = Self::offset(c, dc) else {
                continue;
            };
            if tr >= 10 || tc >= 9 {
                continue;
            }

            // Check leg
            let Some(leg_r) = Self::offset(r, if dr.abs() == 2 { dr / 2 } else { 0 }) else {
                continue;
            };
            let Some(leg_c) = Self::offset(c, if dc.abs() == 2 { dc / 2 } else { 0 }) else {
                continue;
            };

            let leg_sq = Board::square_index(leg_r, leg_c);
            if (board.occupied & (1 << leg_sq)) == 0 {
                self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_elephant_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let offsets = [(-2, -2), (-2, 2), (2, -2), (2, 2)];
        for (dr, dc) in offsets {
            let Some(tr) = Self::offset(r, dr) else {
                continue;
            };
            let Some(tc) = Self::offset(c, dc) else {
                continue;
            };
            if tr >= 10 || tc >= 9 {
                continue;
            }

            // River check
            if turn == Color::Red && tr > 4 {
                continue;
            }
            if turn == Color::Black && tr < 5 {
                continue;
            }

            // Eye check
            let Some(eye_r) = Self::offset(r, dr / 2) else {
                continue;
            };
            let Some(eye_c) = Self::offset(c, dc / 2) else {
                continue;
            };

            let eye_sq = Board::square_index(eye_r, eye_c);
            if (board.occupied & (1 << eye_sq)) == 0 {
                self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_advisor_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let offsets = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
        for (dr, dc) in offsets {
            let Some(tr) = Self::offset(r, dr) else {
                continue;
            };
            let Some(tc) = Self::offset(c, dc) else {
                continue;
            };
            if tr >= 10 || tc >= 9 {
                continue;
            }

            // Palace check
            if !(3..=5).contains(&tc) {
                continue;
            }
            if turn == Color::Red && tr > 2 {
                continue;
            }
            if turn == Color::Black && tr < 7 {
                continue;
            }

            self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_king_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let offsets = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dr, dc) in offsets {
            let Some(tr) = Self::offset(r, dr) else {
                continue;
            };
            let Some(tc) = Self::offset(c, dc) else {
                continue;
            };
            if tr >= 10 || tc >= 9 {
                continue;
            }

            // Palace check
            if !(3..=5).contains(&tc) {
                continue;
            }
            if turn == Color::Red && tr > 2 {
                continue;
            }
            if turn == Color::Black && tr < 7 {
                continue;
            }

            self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_pawn_moves(
        &self,
        board: &mut Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let forward = if turn == Color::Red { 1 } else { -1 };

        // Forward
        let tr = Self::offset(r, forward).unwrap_or(10);
        if tr < 10 {
            self.add_move(board, turn, (r, c), (tr, c), moves, bm, k, oc);
        }

        // Horizontal (if crossed river)
        let crossed_river = if turn == Color::Red { r > 4 } else { r < 5 };
        if crossed_river {
            for dc in [-1, 1] {
                let tc = Self::offset(c, dc).unwrap_or(9);
                if tc < 9 {
                    self.add_move(board, turn, (r, c), (r, tc), moves, bm, k, oc);
                }
            }
        }
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
        excluded_moves: &[Move],
    ) -> Option<(Move, SearchStats)> {
        self.nodes_searched = 0;
        self.start_time = Self::now();
        self.history_stack.clear();

        let (max_depth, time_limit) = match limit {
            SearchLimit::Depth(d) => (d.min(63), None),
            #[allow(clippy::cast_precision_loss)]
            SearchLimit::Time(t) => (64, Some(t as f64)), // Max depth 64 (was 20)
        };
        self.time_limit = time_limit;
        let soft_limit = time_limit.map(|t| t * 0.6);

        let mut board = game_state.board.clone();
        let board = &mut board;
        let turn = game_state.turn;

        // Initialize history stack
        self.history_stack.clear();
        for record in &game_state.history {
            self.history_stack.push(record.hash);
        }

        let mut best_move = None;
        let mut final_depth = 0;
        let mut previous_score: Option<i32> = None;

        // History Aging
        // Decay history scores to adapt to new positions
        for row in self.history_table.iter_mut() {
            for val in row.iter_mut() {
                *val /= 2;
            }
        }

        for d in 1..=max_depth {
            // Check soft limit before starting new depth
            if let Some(sl) = soft_limit {
                let elapsed = Self::now() - self.start_time;
                if elapsed > sl {
                    break;
                }
            }

            let mut alpha = -200000;
            let mut beta = 200000;
            let mut delta = 50;

            if let Some(score) = previous_score {
                if d >= 3 {
                    alpha = (score - delta).max(-200000);
                    beta = (score + delta).min(200000);
                }
            }

            loop {
                let alpha_orig = alpha;
                let beta_orig = beta;
                let mut best_score_this_iteration = -200000;
                let mut current_best_move_this_iteration = None;

                // Try to get best move from TT for this depth (or previous)
                let hash = board.zobrist_hash;
                let tt_move = self.tt.get_move(hash);

                let mut moves = self.generate_moves(board, turn, tt_move, d);

                // Filter excluded moves at root
                if !excluded_moves.is_empty() {
                    moves.retain(|m| {
                        !excluded_moves.iter().any(|ex| {
                            m.from_row == ex.from_row
                                && m.from_col == ex.from_col
                                && m.to_row == ex.to_row
                                && m.to_col == ex.to_col
                        })
                    });
                }

                // Filter for legal moves immediately to handle single-move exception
                moves.retain(|mv| {
                    let captured = board.get_piece(unsafe { BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize) });
                    board.apply_move(mv, turn);
                    let legal = !crate::logic::rules::is_in_check(board, turn)
                        && !crate::logic::rules::is_flying_general(board);
                    board.undo_move(mv, captured, turn);
                    legal
                });

                let is_single_move = moves.len() == 1;

                if self.check_time() {
                    break;
                }

                let mut time_out = false;
                let mut moves_searched = 0;

                for mv in moves {
                    let captured = board.get_piece(unsafe { BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize) });
                    board.apply_move(&mv, turn);

                    if !is_single_move {
                        // 3-Fold Repetition Check at Root
                        // Check if this position has occurred 2 times before (so this is the 3rd)
                        let mut rep_count = 0;
                        for &h in &self.history_stack {
                            if h == board.zobrist_hash {
                                rep_count += 1;
                            }
                        }

                        if rep_count >= 2 {
                            board.undo_move(&mv, captured, turn);
                            continue;
                        }
                    }

                    // Absolute Checkmate Detection at Root - REMOVED for performance
                    // Trust the search to find mates.

                    let score_opt;
                    if moves_searched == 0 {
                        score_opt =
                            self.alpha_beta(board, -beta, -alpha, d - 1, turn.opposite(), None);
                    } else {
                        // Root PVS
                        let mut val = self.alpha_beta(
                            board,
                            -alpha - 1,
                            -alpha,
                            d - 1,
                            turn.opposite(),
                            None,
                        );
                        if let Some(v) = val {
                            let s = -v;
                            if s > alpha && s < beta {
                                val = self.alpha_beta(
                                    board,
                                    -beta,
                                    -alpha,
                                    d - 1,
                                    turn.opposite(),
                                    None,
                                );
                            }
                        }
                        score_opt = val;
                    }

                    board.undo_move(&mv, captured, turn);

                    if let Some(s) = score_opt {
                        let score = -s;
                        if score > best_score_this_iteration {
                            best_score_this_iteration = score;
                            current_best_move_this_iteration = Some(mv);
                        }
                        if score > alpha {
                            alpha = score;
                        }
                    } else {
                        time_out = true;
                        break;
                    }
                    moves_searched += 1;
                }

                if time_out {
                    // If we timed out during a depth, don't use partial results unless we have nothing else
                    if best_move.is_none() && current_best_move_this_iteration.is_some() {
                        best_move = current_best_move_this_iteration;
                        final_depth = d;
                    }
                    break;
                }

                if best_score_this_iteration <= alpha_orig {
                    // Fail Low
                    alpha = (alpha_orig.saturating_sub(delta)).max(-200000);
                    delta = delta.saturating_add(delta / 2);
                    continue;
                }
                if best_score_this_iteration >= beta_orig {
                    // Fail High
                    beta = (beta_orig.saturating_add(delta)).min(200000);
                    delta = delta.saturating_add(delta / 2);
                    continue;
                }

                if let Some(mv) = current_best_move_this_iteration {
                    best_move = Some(mv);
                    final_depth = d;
                    previous_score = Some(best_score_this_iteration);
                }
                break;
            }

            if self.check_time() {
                break;
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
