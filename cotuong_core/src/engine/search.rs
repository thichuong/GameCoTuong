use crate::engine::config::EngineConfig;
use crate::engine::eval::SimpleEvaluator;
use crate::engine::movegen::EngineMoveGen;
use crate::engine::tt::{TTEntry, TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, BoardCoordinate, Color};
use crate::logic::game::GameState;
use crate::logic::rules::{is_flying_general, is_in_check};
use std::sync::Arc;

const MATE_THRESHOLD: i32 = 15_000;
const INF_SCORE: i32 = 500_000;
pub const MAX_PLY: u8 = 96;

pub struct AlphaBetaEngine {
    config: Arc<EngineConfig>,
    evaluator: SimpleEvaluator,
    tt: TranspositionTable,
    history_stack: Vec<u64>,
    game_history_len: usize,
    in_check_stack: [bool; 96],
    killer_moves: [[Option<Move>; 2]; 64],
    history_table: Box<[[i32; 90]; 90]>,
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
    lmr_table: [[u8; 64]; 64],
    mate_score_table: [i32; 256],
}

impl AlphaBetaEngine {
    #[must_use]
    pub fn new(config: Arc<EngineConfig>) -> Self {
        let mate_score_table = Self::precompute_mate_scores(&config);
        let tt_size = if config.tt_size_mb == 0 {
            64
        } else {
            config.tt_size_mb
        };
        Self {
            evaluator: SimpleEvaluator::new(config.clone()),
            tt: TranspositionTable::new(tt_size),
            history_stack: Vec::with_capacity(128),
            game_history_len: 0,
            in_check_stack: [false; 96],
            killer_moves: [[None; 2]; 64],
            history_table: Box::new([[0; 90]; 90]),
            nodes_searched: 0,
            start_time: 0.0,
            time_limit: None,
            lmr_table: Self::precompute_lmr(),
            mate_score_table,
            config,
        }
    }

    #[must_use]
    pub fn with_weights(
        config: Arc<EngineConfig>,
        weights: Arc<crate::engine::nnue::NnueWeights>,
    ) -> Self {
        let mut engine = Self::new(config.clone());
        engine.evaluator = SimpleEvaluator::with_weights(config, weights);
        engine
    }

    pub fn set_eval_weights(&mut self, weights: Option<Arc<crate::engine::nnue::NnueWeights>>) {
        self.evaluator.set_weights(weights);
    }

    pub fn update_config(&mut self, config: Arc<EngineConfig>) {
        if config.tt_size_mb != self.config.tt_size_mb && config.tt_size_mb > 0 {
            self.tt = TranspositionTable::new(config.tt_size_mb);
        }
        self.mate_score_table = Self::precompute_mate_scores(&config);
        self.evaluator = SimpleEvaluator::new(config.clone());
        self.config = config;
    }

    fn precompute_lmr() -> [[u8; 64]; 64] {
        let mut table = [[0; 64]; 64];
        for (depth, row) in table.iter_mut().enumerate() {
            for (moves_searched, val) in row.iter_mut().enumerate() {
                if depth >= 2 && moves_searched >= 2 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        // Standard logarithmic LMR table
                        let r = 0.75
                            + (f64::from(depth as u32).ln()
                                * f64::from(moves_searched as u32).ln())
                                / 2.25;
                        *val = (r as u8).min((depth - 1) as u8);
                    }
                } else {
                    *val = 0;
                }
            }
        }
        table
    }

    fn precompute_mate_scores(config: &EngineConfig) -> [i32; 256] {
        let mut table = [0; 256];
        let base = config.mate_score;
        for (ply, val) in table.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            {
                *val = base - (ply as i32);
            }
        }
        table
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
            0.0
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

    #[inline]
    fn check_time(&self) -> bool {
        if let Some(limit) = self.time_limit {
            if self.nodes_searched.is_multiple_of(1024) {
                let elapsed = Self::now() - self.start_time;
                if elapsed > limit {
                    return true;
                }
            }
        }
        false
    }

    /// Checks for repetitions along the current search branch and within game history.
    /// In Xiangqi:
    /// - Perpetual Check (Trường Chiếu): If one player gives continuous checks in the repetition cycle,
    ///   that checking player is penalized with an immediate LOSS (-MATE_SCORE + ply).
    ///   If opponent gave continuous checks, it's evaluated as a WIN (+MATE_SCORE - ply).
    /// - Standard cycle / draw repetition: Evaluated as 0 (Draw).
    #[inline]
    fn check_repetition(&self, hash: u64, ply: u8) -> Option<i32> {
        if ply == 0 {
            return None;
        }

        let curr_idx = self.history_stack.len();
        let search_path_start = self.game_history_len;

        // 1. Check repetition within current search path (from ply - 1 back to root ply 0)
        if curr_idx > search_path_start {
            for idx in (search_path_start..curr_idx).rev() {
                if self.history_stack[idx] == hash {
                    let ancestor_ply = (idx - search_path_start) as u8;

                    let mut checker_is_current_player = true;
                    let mut checker_is_opponent = true;

                    // Moves made by current player resulted in states at ancestor_ply + 1, ancestor_ply + 3, ...
                    let mut p = ancestor_ply + 1;
                    while p <= ply {
                        if !self.in_check_stack[(p as usize).min(95)] {
                            checker_is_current_player = false;
                        }
                        p += 2;
                    }

                    // Moves made by opponent resulted in states at ancestor_ply + 2, ancestor_ply + 4, ...
                    let mut q = ancestor_ply + 2;
                    while q <= ply {
                        if !self.in_check_stack[(q as usize).min(95)] {
                            checker_is_opponent = false;
                        }
                        q += 2;
                    }

                    if checker_is_current_player {
                        // Current player committed Perpetual Check -> ILLEGAL / LOSS in Xiangqi
                        return Some(-self.calculate_mate_score(ply));
                    }
                    if checker_is_opponent {
                        // Opponent committed Perpetual Check -> Current player WINS
                        return Some(self.calculate_mate_score(ply));
                    }

                    // Normal repetition cycle -> Draw (0)
                    return Some(0);
                }
            }
        }

        // 2. Check repetition against past game history
        if search_path_start > 0 {
            let hist_count = self.history_stack[..search_path_start]
                .iter()
                .filter(|&&h| h == hash)
                .count();

            // If already occurred 2 or more times in actual game history, 3rd time is draw
            if hist_count >= 2 {
                return Some(0);
            }
        }

        None
    }

    #[inline]
    fn calculate_mate_score(&self, ply: u8) -> i32 {
        self.mate_score_table
            .get(ply as usize)
            .copied()
            .unwrap_or(self.config.mate_score - i32::from(ply))
    }

    #[cfg(test)]
    #[must_use]
    pub fn calculate_mate_score_for_test(&self, ply: u8) -> i32 {
        self.calculate_mate_score(ply)
    }

    #[inline]
    fn eval_for_turn(&self, board: &Board, turn: Color) -> i32 {
        let raw = self.evaluator.evaluate(board);
        if turn == Color::Red {
            raw
        } else {
            -raw
        }
    }

    #[inline]
    fn refine_eval_with_tt(static_eval: i32, tt_entry: Option<TTEntry>) -> i32 {
        if let Some(entry) = tt_entry {
            if entry.score.abs() < MATE_THRESHOLD {
                match entry.flag {
                    TTFlag::Exact => return entry.score,
                    TTFlag::LowerBound if entry.score > static_eval => return entry.score,
                    TTFlag::UpperBound if entry.score < static_eval => return entry.score,
                    _ => {}
                }
            }
        }
        static_eval
    }

    /// Pikafish & Fairy-Stockfish standard Null Move Pruning (NMP)
    fn null_move_pruning(
        &mut self,
        board: &mut Board,
        eval: i32,
        beta: i32,
        depth: u8,
        turn: Color,
        ply: u8,
    ) -> Option<Option<i32>> {
        if depth >= 3
            && eval >= beta
            && beta.abs() < MATE_THRESHOLD
            && board.has_major_pieces(turn)
            && ply.saturating_add(1) < MAX_PLY
        {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let eval_bonus = ((eval - beta) / 200).clamp(0, 3) as u8;
            let r = (3 + depth / 4 + eval_bonus).min(depth - 1);

            board.apply_null_move();
            let result = self.alpha_beta(
                board,
                -beta,
                -beta + 1,
                depth.saturating_sub(r),
                turn.opposite(),
                ply.saturating_add(1),
            );
            board.apply_null_move();

            if let Some(score) = result {
                let nmp_score = -score;
                if nmp_score >= beta {
                    return Some(Some(if nmp_score >= MATE_THRESHOLD {
                        beta
                    } else {
                        nmp_score
                    }));
                }
            } else {
                return Some(None);
            }
        }
        None
    }

    fn quiescence(
        &mut self,
        board: &mut Board,
        mut alpha: i32,
        beta: i32,
        turn: Color,
        ply: u8,
    ) -> Option<i32> {
        if ply >= MAX_PLY {
            return Some(self.eval_for_turn(board, turn));
        }

        self.nodes_searched += 1;
        if self.check_time() {
            return None;
        }

        let in_check = is_in_check(board, turn);
        self.in_check_stack[(ply as usize).min(95)] = in_check;
        if !in_check {
            let stand_pat = self.eval_for_turn(board, turn);
            if stand_pat >= beta {
                return Some(beta);
            }
            if stand_pat > alpha {
                alpha = stand_pat;
            }
        }

        let movegen = EngineMoveGen::new();
        let moves = if in_check {
            movegen.generate_moves(board, turn, None)
        } else {
            movegen.generate_captures(board, turn)
        };

        let mut legal_moves_count = 0;
        for mv in moves.iter() {
            let Some((piece, captured, from_sq, to_sq)) = board.apply_move_board_only(mv) else {
                continue;
            };

            if is_in_check(board, turn) || is_flying_general(board) {
                board.undo_move_board_only(mv, piece, captured);
                continue;
            }

            legal_moves_count += 1;
            let saved_acc = board.nnue_acc;
            board.update_nnue_for_move_with_weights(
                from_sq,
                to_sq,
                piece,
                captured,
                self.evaluator.weights(),
            );

            let score_opt = self
                .quiescence(board, -beta, -alpha, turn.opposite(), ply.saturating_add(1))
                .map(|v| -v);

            board.undo_move_board_only(mv, piece, captured);
            board.nnue_acc = saved_acc;

            let score = score_opt?;

            if score >= beta {
                return Some(beta);
            }
            if score > alpha {
                alpha = score;
            }
        }

        if in_check && legal_moves_count == 0 {
            return Some(-self.calculate_mate_score(ply));
        }

        Some(alpha)
    }

    #[allow(clippy::too_many_lines)]
    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: i32,
        mut beta: i32,
        mut depth: u8,
        turn: Color,
        ply: u8,
    ) -> Option<i32> {
        if ply >= MAX_PLY {
            return Some(self.eval_for_turn(board, turn));
        }

        self.nodes_searched += 1;

        if self.check_time() {
            return None;
        }

        let is_pv = beta - alpha > 1;

        // 1. Mate Distance Pruning
        if ply > 0 {
            let mat = self.config.mate_score - i32::from(ply);
            alpha = alpha.max(-mat);
            beta = beta.min(mat);
            if alpha >= beta {
                return Some(alpha);
            }
        }

        let hash = board.zobrist_hash;
        if let Some(rep_score) = self.check_repetition(hash, ply) {
            return Some(rep_score);
        }

        self.history_stack.push(hash);

        // 2. Transposition Table Probe
        let tt_entry = self.tt.probe(hash);
        if let Some(entry) = tt_entry {
            if entry.depth >= depth && (!is_pv || entry.flag == TTFlag::Exact) {
                match entry.flag {
                    TTFlag::Exact => {
                        self.history_stack.pop();
                        return Some(entry.score);
                    }
                    TTFlag::LowerBound if entry.score >= beta => {
                        self.history_stack.pop();
                        return Some(entry.score);
                    }
                    TTFlag::UpperBound if entry.score <= alpha => {
                        self.history_stack.pop();
                        return Some(entry.score);
                    }
                    TTFlag::LowerBound => alpha = alpha.max(entry.score),
                    TTFlag::UpperBound => beta = beta.min(entry.score),
                }
                if alpha >= beta {
                    self.history_stack.pop();
                    return Some(entry.score);
                }
            }
        }

        // Quiescence Search at depth 0
        if depth == 0 {
            let eval = self.quiescence(board, alpha, beta, turn, ply);
            self.history_stack.pop();
            return eval;
        }

        let in_check = is_in_check(board, turn);
        self.in_check_stack[(ply as usize).min(95)] = in_check;

        // 3. Non-PV Pruning: Reverse Futility Pruning (RFP) & Null Move Pruning (NMP)
        if !is_pv && !in_check {
            let raw_eval = self.eval_for_turn(board, turn);
            let eval = Self::refine_eval_with_tt(raw_eval, tt_entry);

            // Reverse Futility Pruning (Static Null Move Pruning)
            if depth <= 6 && beta.abs() < MATE_THRESHOLD {
                let rfp_margin = 90 * i32::from(depth);
                if eval - rfp_margin >= beta {
                    self.history_stack.pop();
                    return Some((eval + beta) / 2);
                }
            }

            // Null Move Pruning
            if let Some(res) = self.null_move_pruning(board, eval, beta, depth, turn, ply) {
                self.history_stack.pop();
                return res;
            }
        }

        let best_move_tt = tt_entry.and_then(|e| e.best_move);

        // Internal Iterative Reductions (IIR)
        if depth >= 4 && best_move_tt.is_none() {
            depth -= 1;
        }

        let p_idx = (ply as usize).min(63);
        let killers = self.killer_moves[p_idx];

        let movegen = EngineMoveGen::new();
        let moves = movegen.generate_moves_with_ordering(
            board,
            turn,
            best_move_tt,
            &killers,
            Some(&*self.history_table),
        );

        let mut best_score = -INF_SCORE;
        let mut best_move_this_node = None;
        let mut tt_flag = TTFlag::UpperBound;
        let mut legal_moves_count = 0usize;

        let full_depth = depth.saturating_sub(1) + if in_check { 1 } else { 0 };

        for mv in moves.iter() {
            let Some((piece, captured, from_sq, to_sq)) = board.apply_move_board_only(mv) else {
                continue;
            };

            // Fast legal validation: self-check and flying general
            if is_in_check(board, turn) || is_flying_general(board) {
                board.undo_move_board_only(mv, piece, captured);
                continue;
            }

            legal_moves_count += 1;
            let is_capture = captured.is_some();

            // Lazy NNUE update: save accumulator on stack, update for this move
            let saved_acc = board.nnue_acc;
            board.update_nnue_for_move_with_weights(
                from_sq,
                to_sq,
                piece,
                captured,
                self.evaluator.weights(),
            );

            // Late Move Reductions (LMR)
            let mut reduction = 0u8;
            if depth >= 3 && legal_moves_count >= 3 && !in_check && !is_capture {
                let d = (depth as usize).min(63);
                let m = legal_moves_count.min(63);
                let r = self
                    .lmr_table
                    .get(d)
                    .and_then(|row| row.get(m))
                    .copied()
                    .unwrap_or(0);
                reduction = if !is_pv { r + 1 } else { r }.min(full_depth.saturating_sub(1));
            }

            let next_ply = ply.saturating_add(1);
            let score_opt = if legal_moves_count == 1 || full_depth == 0 {
                self.alpha_beta(board, -beta, -alpha, full_depth, turn.opposite(), next_ply)
                    .map(|v| -v)
            } else {
                let reduced_depth = full_depth.saturating_sub(reduction);
                let mut val = if reduction > 0 && reduced_depth > 0 {
                    self.alpha_beta(
                        board,
                        -alpha - 1,
                        -alpha,
                        reduced_depth,
                        turn.opposite(),
                        next_ply,
                    )
                    .map(|v| -v)
                } else {
                    None
                };

                if val.is_none_or(|s| s > alpha) {
                    val = self
                        .alpha_beta(
                            board,
                            -alpha - 1,
                            -alpha,
                            full_depth,
                            turn.opposite(),
                            next_ply,
                        )
                        .map(|v| -v);
                }

                if let Some(s) = val {
                    if s > alpha && s < beta {
                        val = self
                            .alpha_beta(board, -beta, -alpha, full_depth, turn.opposite(), next_ply)
                            .map(|v| -v);
                    }
                }
                val
            };

            board.undo_move_board_only(mv, piece, captured);
            board.nnue_acc = saved_acc;

            let Some(score) = score_opt else {
                self.history_stack.pop();
                return None;
            };

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(*mv);
            }
            if score > alpha {
                alpha = score;
                tt_flag = TTFlag::Exact;
            }

            if alpha >= beta {
                tt_flag = TTFlag::LowerBound;
                if !is_capture {
                    if self.killer_moves[p_idx][0] != Some(*mv) {
                        self.killer_moves[p_idx][1] = self.killer_moves[p_idx][0];
                        self.killer_moves[p_idx][0] = Some(*mv);
                    }
                    if let Some(row) = self.history_table.get_mut(from_sq) {
                        if let Some(val) = row.get_mut(to_sq) {
                            let bonus = i32::from(depth) * i32::from(depth);
                            *val = (*val + bonus).min(50_000);
                        }
                    }
                }
                break;
            }
        }

        if legal_moves_count == 0 {
            self.history_stack.pop();
            return Some(-self.calculate_mate_score(ply));
        }

        self.tt
            .store(hash, best_move_this_node, best_score, depth, tt_flag);

        self.history_stack.pop();
        Some(best_score)
    }
}

impl Searcher for AlphaBetaEngine {
    #[allow(clippy::too_many_lines)]
    fn search(
        &mut self,
        game_state: &GameState,
        limit: SearchLimit,
        excluded_moves: &[Move],
    ) -> Option<(Move, SearchStats)> {
        self.nodes_searched = 0;
        self.start_time = Self::now();
        self.history_stack.clear();
        self.killer_moves = [[None; 2]; 64];
        *self.history_table = [[0; 90]; 90];

        let (max_depth, time_limit) = match limit {
            SearchLimit::Depth(d) => (d.min(63), None),
            #[allow(clippy::cast_precision_loss)]
            SearchLimit::Time(t) => (64, Some(t as f64)),
            #[allow(clippy::cast_precision_loss)]
            SearchLimit::DepthOrTime { depth, time_ms } => (depth.min(63), Some(time_ms as f64)),
        };
        self.time_limit = time_limit;
        let soft_limit = time_limit.map(|t| t * 0.6);

        let mut board = game_state.board.clone();
        board.refresh_nnue_with_weights(self.evaluator.weights());
        let board = &mut board;
        let turn = game_state.turn;

        self.history_stack.clear();
        self.history_stack.push(game_state.initial_hash);
        for record in &game_state.history {
            self.history_stack.push(record.hash);
        }
        self.game_history_len = self.history_stack.len();
        self.in_check_stack = [false; 96];

        // 1. Pre-filter strictly legal, non-repetition moves at Root ("bước đầu")
        let mut root_moves = game_state.get_strictly_legal_moves();
        if !excluded_moves.is_empty() {
            root_moves.retain(|m| {
                !excluded_moves.iter().any(|ex| {
                    m.from_row == ex.from_row
                        && m.from_col == ex.from_col
                        && m.to_row == ex.to_row
                        && m.to_col == ex.to_col
                })
            });
        }

        if root_moves.is_empty() {
            return None;
        }

        if root_moves.len() == 1 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let elapsed_ms = (Self::now() - self.start_time).max(0.0) as u64;
            return Some((
                root_moves[0],
                SearchStats {
                    depth: 1,
                    nodes: 1,
                    time_ms: elapsed_ms,
                    score: 0,
                },
            ));
        }

        let mut best_move = None;
        let mut final_depth = 0;
        let mut previous_score: Option<i32> = None;

        for d in 1..=max_depth {
            if let Some(sl) = soft_limit {
                let elapsed = Self::now() - self.start_time;
                if elapsed > sl {
                    break;
                }
            }

            let mut alpha = -INF_SCORE;
            let mut beta = INF_SCORE;
            let mut delta = 35;

            if let Some(score) = previous_score {
                if d >= 4 && score.abs() < MATE_THRESHOLD {
                    alpha = (score - delta).max(-INF_SCORE);
                    beta = (score + delta).min(INF_SCORE);
                }
            }

            loop {
                let alpha_orig = alpha;
                let beta_orig = beta;
                let mut best_score_this_iteration = -INF_SCORE;
                let mut current_best_move_this_iteration = None;

                let hash = board.zobrist_hash;
                let tt_move = best_move.or_else(|| self.tt.get_move(hash));

                let movegen = EngineMoveGen::new();
                let mut moves = movegen.generate_moves_with_ordering(
                    board,
                    turn,
                    tt_move,
                    &self.killer_moves[0],
                    Some(&*self.history_table),
                );

                // Filter candidate moves to only strictly legal, non-repetition root moves
                moves.retain(|mv| {
                    root_moves.iter().any(|rm| {
                        rm.from_row == mv.from_row
                            && rm.from_col == mv.from_col
                            && rm.to_row == mv.to_row
                            && rm.to_col == mv.to_col
                    })
                });

                if moves.is_empty() {
                    break;
                }

                if self.check_time() {
                    break;
                }

                let mut time_out = false;

                for (moves_searched, mv) in moves.iter().enumerate() {
                    let Some((piece, captured, from_sq, to_sq)) = board.apply_move_board_only(mv)
                    else {
                        continue;
                    };
                    let saved_acc = board.nnue_acc;
                    board.update_nnue_for_move_with_weights(
                        from_sq,
                        to_sq,
                        piece,
                        captured,
                        self.evaluator.weights(),
                    );

                    let score_opt = if moves_searched == 0 {
                        self.alpha_beta(board, -beta, -alpha, d - 1, turn.opposite(), 1)
                    } else {
                        let mut val =
                            self.alpha_beta(board, -alpha - 1, -alpha, d - 1, turn.opposite(), 1);
                        if let Some(v) = val {
                            let s = -v;
                            if s > alpha && s < beta {
                                val = self.alpha_beta(
                                    board,
                                    -beta,
                                    -alpha,
                                    d - 1,
                                    turn.opposite(),
                                    1,
                                );
                            }
                        }
                        val
                    };

                    board.undo_move_board_only(mv, piece, captured);
                    board.nnue_acc = saved_acc;

                    if let Some(s) = score_opt {
                        let score = -s;
                        if score > best_score_this_iteration {
                            best_score_this_iteration = score;
                            current_best_move_this_iteration = Some(*mv);
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
                    if best_move.is_none() && current_best_move_this_iteration.is_some() {
                        best_move = current_best_move_this_iteration;
                        final_depth = d;
                    }
                    break;
                }

                if best_score_this_iteration <= alpha_orig {
                    if alpha_orig <= -INF_SCORE {
                        break;
                    }
                    alpha = (alpha_orig.saturating_sub(delta)).max(-INF_SCORE);
                    delta = delta.saturating_add(delta / 2);
                    continue;
                }
                if best_score_this_iteration >= beta_orig {
                    if beta_orig >= INF_SCORE {
                        break;
                    }
                    beta = (beta_orig.saturating_add(delta)).min(INF_SCORE);
                    delta = delta.saturating_add(delta / 2);
                    continue;
                }

                if let Some(mv) = current_best_move_this_iteration {
                    best_move = Some(mv);
                    final_depth = d;
                    previous_score = Some(best_score_this_iteration);
                    self.tt.store(
                        board.zobrist_hash,
                        Some(mv),
                        best_score_this_iteration,
                        d,
                        TTFlag::Exact,
                    );
                }
                break;
            }

            if self.check_time() {
                break;
            }
        }

        let strictly_legal_moves = game_state.get_strictly_legal_moves();
        if let Some(mv) = best_move {
            let (Some(from), Some(to)) = (
                BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize),
                BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize),
            ) else {
                best_move = strictly_legal_moves.first().copied();
                return best_move.map(|valid_mv| {
                    (
                        valid_mv,
                        SearchStats {
                            depth: final_depth,
                            nodes: self.nodes_searched,
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            time_ms: (Self::now() - self.start_time) as u64,
                            score: previous_score.unwrap_or(0),
                        },
                    )
                });
            };

            if !game_state.is_move_legal(from, to) {
                best_move = strictly_legal_moves.first().copied();
            }
        } else {
            best_move = strictly_legal_moves.first().copied();
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
                    score: previous_score.unwrap_or(0),
                },
            )
        })
    }
}
