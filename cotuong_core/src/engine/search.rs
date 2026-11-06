use crate::engine::config::EngineConfig;
use crate::engine::eval::SimpleEvaluator;
use crate::engine::movegen::{EngineMoveGen, SCORE_KILLER_MOVE};
use crate::engine::tt::{TTEntry, TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, BoardCoordinate, Color};
use crate::logic::eval_constants::get_piece_value;
use crate::logic::game::GameState;
use crate::logic::generator::MoveGenerator;
use crate::logic::rules::{is_flying_general, is_in_check};
use std::sync::Arc;

const MATE_THRESHOLD: i32 = 15_000;
const INF_SCORE: i32 = 500_000;
pub const MAX_PLY: u8 = 96;

pub struct AlphaBetaEngine {
    config: Arc<EngineConfig>,
    evaluator: SimpleEvaluator,
    tt: TranspositionTable,
    killer_moves: [[Option<Move>; 2]; 64],
    countermove_table: Box<[[Option<Move>; 90]; 90]>,
    history_stack: Vec<u64>,
    pub history_table: Box<[[i32; 90]]>,
    static_eval_stack: [i32; 128],
    move_stack: [Option<Move>; 128],
    nodes_searched: u32,
    start_time: f64,
    time_limit: Option<f64>,
    dynamic_limits: [usize; 64],
    lmr_table: [[u8; 64]; 64],
    mate_score_table: [i32; 256],
}

impl AlphaBetaEngine {
    #[must_use]
    pub fn new(config: Arc<EngineConfig>) -> Self {
        let dynamic_limits = Self::precompute_limits(&config);
        let mate_score_table = Self::precompute_mate_scores(&config);
        let Ok(countermove_table) = vec![[None; 90]; 90].into_boxed_slice().try_into() else {
            unreachable!()
        };
        Self {
            evaluator: SimpleEvaluator::new(config.clone()),
            config,
            tt: TranspositionTable::new(64),
            killer_moves: [[None; 2]; 64],
            countermove_table,
            history_stack: Vec::with_capacity(64),
            history_table: vec![[0; 90]; 90].into_boxed_slice(),
            static_eval_stack: [0; 128],
            move_stack: [None; 128],
            nodes_searched: 0,
            start_time: 0.0,
            time_limit: None,
            dynamic_limits,
            lmr_table: Self::precompute_lmr(),
            mate_score_table,
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
        if config.tt_size_mb != self.config.tt_size_mb {
            self.tt = TranspositionTable::new(config.tt_size_mb);
        }
        self.dynamic_limits = Self::precompute_limits(&config);
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
                        // Pikafish / Fairy-Stockfish logarithmic LMR base table
                        let r = 0.75
                            + (f64::from(depth as u32).ln()
                                * f64::from(moves_searched as u32).ln())
                                / 2.15;
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
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_sign_loss,
                clippy::cast_possible_truncation
            )]
            {
                let depth = d as f32;
                let multiplier = config.pruning_multiplier;
                *limit = (depth * depth).mul_add(multiplier, 8.0) as usize;
            }
        }
        limits
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

    fn is_repetition(&self, hash: u64) -> bool {
        self.history_stack.iter().filter(|&&h| h == hash).count() >= 2
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

    fn probcut(
        &mut self,
        board: &mut Board,
        beta: i32,
        depth: u8,
        turn: Color,
        ply: u8,
    ) -> Option<Option<i32>> {
        if depth >= self.config.probcut_depth
            && beta.abs() < MATE_THRESHOLD
            && ply.saturating_add(1) < MAX_PLY
        {
            let margin = self.config.probcut_margin;
            let reduction = self.config.probcut_reduction;

            if depth > reduction {
                let probcut_beta = beta.saturating_add(margin);
                if let Some(score) = self.alpha_beta(
                    board,
                    -probcut_beta,
                    -probcut_beta + 1,
                    depth - reduction,
                    turn.opposite(),
                    ply.saturating_add(1),
                    None,
                ) {
                    if -score >= probcut_beta {
                        return Some(Some(beta));
                    }
                } else {
                    return Some(None);
                }
            }
        }
        None
    }

    /// Pikafish & Fairy-Stockfish Null Move Pruning with dynamic eval-based reduction
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
            let eval_bonus = ((eval - beta) / 180).clamp(0, 3) as u8;
            let r = (3 + depth / 4 + eval_bonus).min(depth - 1);

            board.apply_null_move();
            if let Some(slot) = self.move_stack.get_mut(ply as usize) {
                *slot = None;
            }

            let result = self.alpha_beta(
                board,
                -beta,
                -beta + 1,
                depth.saturating_sub(r),
                turn.opposite(),
                ply.saturating_add(1),
                None,
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

    #[allow(clippy::too_many_arguments)]
    fn singular_extension(
        &mut self,
        board: &mut Board,
        tt_entry: Option<TTEntry>,
        depth: u8,
        turn: Color,
        ply: u8,
        best_move_tt: Option<Move>,
        excluded_move: Option<Move>,
    ) -> u8 {
        if depth >= self.config.singular_extension_min_depth
            && excluded_move.is_none()
            && best_move_tt.is_some()
            && ply.saturating_add(1) < MAX_PLY
        {
            if let Some(entry) = tt_entry {
                if entry.depth >= depth.saturating_sub(3)
                    && entry.score.abs() < MATE_THRESHOLD
                    && (entry.flag == TTFlag::Exact || entry.flag == TTFlag::LowerBound)
                {
                    let margin = self.config.singular_extension_margin + 2 * i32::from(depth);
                    let singular_beta = entry.score - margin;

                    if let Some(score) = self.alpha_beta(
                        board,
                        singular_beta - 1,
                        singular_beta,
                        (depth - 1) / 2,
                        turn,
                        ply,
                        best_move_tt,
                    ) {
                        if score < singular_beta {
                            return 1;
                        }
                    }
                }
            }
        }
        0
    }

    /// Pikafish / Stockfish History Gravity update (stat bonus + malus for searched quiet moves)
    fn update_quiet_histories(
        &mut self,
        depth: u8,
        ply: u8,
        best_mv: Move,
        quiet_moves_tried: &[Move],
    ) {
        self.store_killer(depth, best_mv);

        // Store Countermove against previous move
        if ply > 0 {
            if let Some(Some(prev_mv)) = self.move_stack.get((ply - 1) as usize) {
                let p_from = (prev_mv.from_row as usize) * 9 + (prev_mv.from_col as usize);
                let p_to = (prev_mv.to_row as usize) * 9 + (prev_mv.to_col as usize);
                if let Some(row) = self.countermove_table.get_mut(p_from) {
                    if let Some(slot) = row.get_mut(p_to) {
                        *slot = Some(best_mv);
                    }
                }
            }
        }

        let d = i32::from(depth);
        let bonus = (d * d * 16 + 32 * d).min(1600);

        self.apply_history_gravity(best_mv, bonus);
        for &q_mv in quiet_moves_tried {
            if q_mv != best_mv {
                self.apply_history_gravity(q_mv, -bonus);
            }
        }
    }

    #[inline]
    fn apply_history_gravity(&mut self, mv: Move, bonus: i32) {
        let from = (mv.from_row as usize) * 9 + (mv.from_col as usize);
        let to = (mv.to_row as usize) * 9 + (mv.to_col as usize);
        if let Some(row) = self.history_table.get_mut(from) {
            if let Some(entry) = row.get_mut(to) {
                let current = *entry;
                *entry = current + bonus - (current * bonus.abs()) / 16_384;
            }
        }
    }

    #[inline]
    fn get_history_score(&self, mv: Move) -> i32 {
        let from = (mv.from_row as usize) * 9 + (mv.from_col as usize);
        let to = (mv.to_row as usize) * 9 + (mv.to_col as usize);
        self.history_table
            .get(from)
            .and_then(|r| r.get(to))
            .copied()
            .unwrap_or(0)
    }

    #[inline]
    fn get_countermove(&self, ply: u8) -> Option<Move> {
        if ply == 0 {
            return None;
        }
        let prev_mv = (*self.move_stack.get((ply - 1) as usize)?)?;
        let p_from = (prev_mv.from_row as usize) * 9 + (prev_mv.from_col as usize);
        let p_to = (prev_mv.to_row as usize) * 9 + (prev_mv.to_col as usize);
        self.countermove_table
            .get(p_from)
            .and_then(|r| r.get(p_to))
            .copied()
            .flatten()
    }

    #[allow(clippy::too_many_arguments)]
    fn search_pvs_child(
        &mut self,
        board: &mut Board,
        alpha: i32,
        beta: i32,
        depth: u8,
        reduction: u8,
        extension: u8,
        turn: Color,
        ply: u8,
        legal_moves_count: usize,
    ) -> Option<i32> {
        let full_depth = depth.saturating_sub(1).saturating_add(extension);
        if legal_moves_count == 1 {
            return self
                .alpha_beta(
                    board,
                    -beta,
                    -alpha,
                    full_depth,
                    turn.opposite(),
                    ply.saturating_add(1),
                    None,
                )
                .map(|v| -v);
        }

        let reduced_depth = full_depth.saturating_sub(reduction).max(1);
        let mut val = self.alpha_beta(
            board,
            -alpha - 1,
            -alpha,
            reduced_depth,
            turn.opposite(),
            ply.saturating_add(1),
            None,
        );

        if let Some(v) = val {
            let s = -v;
            if s > alpha && reduction > 0 {
                val = self.alpha_beta(
                    board,
                    -alpha - 1,
                    -alpha,
                    full_depth,
                    turn.opposite(),
                    ply.saturating_add(1),
                    None,
                );
            }
            if let Some(v2) = val {
                let s2 = -v2;
                if s2 > alpha && s2 < beta {
                    val = self.alpha_beta(
                        board,
                        -beta,
                        -alpha,
                        full_depth,
                        turn.opposite(),
                        ply.saturating_add(1),
                        None,
                    );
                }
            }
        }

        val.map(|v| -v)
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn alpha_beta(
        &mut self,
        board: &mut Board,
        mut alpha: i32,
        mut beta: i32,
        mut depth: u8,
        turn: Color,
        ply: u8,
        excluded_move: Option<Move>,
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
        if self.is_repetition(hash) {
            return Some(0);
        }

        self.history_stack.push(hash);

        // 2. Transposition Table Probe
        let tt_entry = if excluded_move.is_none() {
            self.tt.probe(hash)
        } else {
            None
        };

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

        if depth == 0 {
            let score = self.quiescence(board, alpha, beta, turn, ply);
            self.history_stack.pop();
            return Some(score);
        }

        let in_check = is_in_check(board, turn);

        // 3. Static Evaluation & Pikafish `improving` Heuristic
        let (static_eval, eval, improving) = if in_check {
            (-INF_SCORE, -INF_SCORE, false)
        } else {
            let raw_eval = self.eval_for_turn(board, turn);
            if let Some(slot) = self.static_eval_stack.get_mut(ply as usize) {
                *slot = raw_eval;
            }
            let refined = Self::refine_eval_with_tt(raw_eval, tt_entry);
            let imp = ply >= 2
                && self
                    .static_eval_stack
                    .get((ply - 2) as usize)
                    .is_some_and(|&prev_eval| prev_eval > -INF_SCORE && raw_eval > prev_eval);
            (raw_eval, refined, imp)
        };

        // 4. Non-PV Pre-Move-Loop Pruning (Pikafish & Fairy-Stockfish Pipeline)
        if !is_pv && !in_check && excluded_move.is_none() {
            // 4a. Razoring
            if depth <= 2 && alpha.abs() < MATE_THRESHOLD {
                let razor_margin = 210 * i32::from(depth);
                if eval + razor_margin <= alpha {
                    let qscore = self.quiescence(board, alpha, beta, turn, ply);
                    if qscore <= alpha {
                        self.history_stack.pop();
                        return Some(qscore);
                    }
                }
            }

            // 4b. Reverse Futility Pruning (RFP / Static Null Move Pruning)
            if depth <= 6 && beta.abs() < MATE_THRESHOLD {
                let rfp_margin = 85 * i32::from(depth) - if improving { 40 } else { 0 };
                if eval - rfp_margin >= beta {
                    self.history_stack.pop();
                    return Some((eval + beta) / 2);
                }
            }

            // 4c. Null Move Pruning (NMP)
            if let Some(res) = self.null_move_pruning(board, eval, beta, depth, turn, ply) {
                self.history_stack.pop();
                return res;
            }

            // 4d. ProbCut
            if let Some(res) = self.probcut(board, beta, depth, turn, ply) {
                self.history_stack.pop();
                return res;
            }
        }

        let best_move_tt = tt_entry.and_then(|e| e.best_move);

        // 5. Internal Iterative Reductions (IIR - Pikafish / Stockfish replacement for IID)
        if depth >= 4 && best_move_tt.is_none() && excluded_move.is_none() {
            depth -= 1;
        }

        let singular_ext = self.singular_extension(
            board,
            tt_entry,
            depth,
            turn,
            ply,
            best_move_tt,
            excluded_move,
        );

        let countermove = self.get_countermove(ply);
        let movegen = EngineMoveGen::new(&self.history_table);
        let mut moves = movegen.generate_moves_with_counter(
            board,
            turn,
            best_move_tt,
            &self.killer_moves,
            countermove,
            depth,
        );

        if in_check {
            moves.retain(|mv| {
                let captured = board.get_piece(unsafe {
                    BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
                });
                board.apply_move(mv, turn);
                let legal = !is_in_check(board, turn) && !is_flying_general(board);
                board.undo_move(mv, captured, turn);
                legal
            });

            if moves.is_empty() {
                self.history_stack.pop();
                return Some(-self.calculate_mate_score(ply));
            }
        } else if moves.is_empty() {
            self.history_stack.pop();
            return Some(-self.calculate_mate_score(ply));
        }

        let dynamic_limit =
            if !in_check && (self.config.pruning_method == 0 || self.config.pruning_method == 2) {
                self.dynamic_limits
                    .get(depth as usize)
                    .copied()
                    .unwrap_or_else(|| moves.len())
            } else {
                moves.len()
            };

        // Pikafish / Fairy-Stockfish Late Move Pruning (LMP) count
        let lmp_limit = if !in_check && depth <= 6 {
            let d = usize::from(depth);
            let base = 4 + d * d;
            if improving {
                base + d * 2
            } else {
                (base * 2) / 3
            }
        } else {
            moves.len()
        };

        let mut best_score = -INF_SCORE;
        let mut best_move_this_node = None;
        let mut tt_flag = TTFlag::UpperBound;
        let mut legal_moves_count = 0usize;
        let mut has_repetition_move = false;
        let mut quiet_moves_tried = [Move::default(); 64];
        let mut quiet_count = 0usize;

        for (moves_searched, mv) in moves.into_iter().enumerate() {
            if excluded_move == Some(mv) {
                continue;
            }

            let is_capture = board
                .get_piece(unsafe {
                    BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
                })
                .is_some();

            // Quiet Move Pruning (LMP, Dynamic Limiting & Futility Pruning)
            if !is_pv && !in_check && !is_capture && best_score > -MATE_THRESHOLD {
                if moves_searched >= dynamic_limit || legal_moves_count >= lmp_limit {
                    continue;
                }

                if depth <= 4
                    && legal_moves_count > 0
                    && (self.config.pruning_method == 0 || self.config.pruning_method == 2)
                {
                    let hist = self.get_history_score(mv);
                    let margin = 100 + 85 * i32::from(depth) + (hist / 128);
                    if static_eval + margin <= alpha {
                        continue;
                    }
                }
            }

            let captured = board.get_piece(unsafe {
                BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
            });
            board.apply_move(&mv, turn);

            if !in_check && (is_in_check(board, turn) || is_flying_general(board)) {
                board.undo_move(&mv, captured, turn);
                continue;
            }

            if !MoveGenerator::new().has_legal_moves(board, turn.opposite()) {
                self.history_stack.pop();
                board.undo_move(&mv, captured, turn);
                return Some(self.calculate_mate_score(ply.saturating_add(1)));
            }

            let rep_count = self
                .history_stack
                .iter()
                .filter(|&&h| h == board.zobrist_hash)
                .count();
            if rep_count >= 2 {
                board.undo_move(&mv, captured, turn);
                has_repetition_move = true;
                continue;
            }

            legal_moves_count += 1;
            if !is_capture {
                if let Some(slot) = quiet_moves_tried.get_mut(quiet_count) {
                    *slot = mv;
                    quiet_count += 1;
                }
            }
            if let Some(slot) = self.move_stack.get_mut(ply as usize) {
                *slot = Some(mv);
            }

            // Pikafish / Fairy-Stockfish Late Move Reductions (LMR)
            let mut reduction = 0u8;
            if depth >= 3
                && legal_moves_count >= 3
                && (self.config.pruning_method == 1 || self.config.pruning_method == 2)
                && !in_check
                && !is_capture
            {
                let d = (depth as usize).min(63);
                let m = legal_moves_count.min(63);
                let mut r = i32::from(
                    self.lmr_table
                        .get(d)
                        .and_then(|row| row.get(m))
                        .copied()
                        .unwrap_or(0),
                );
                if !is_pv {
                    r += 1;
                }
                if !improving {
                    r += 1;
                }
                if mv.score >= SCORE_KILLER_MOVE.saturating_sub(20_000) {
                    r -= 1;
                }
                let hist_adj = (self.get_history_score(mv) / 4096).clamp(-2, 2);
                r -= hist_adj;

                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    reduction = r.clamp(0, i32::from(depth.saturating_sub(2))) as u8;
                }
            }

            let is_tt_move = best_move_tt == Some(mv);
            let extension = if in_check {
                1
            } else if is_tt_move {
                singular_ext
            } else {
                0
            };
            let extension = if ply.saturating_add(4) >= MAX_PLY {
                0
            } else {
                extension
            };

            let Some(score) = self.search_pvs_child(
                board,
                alpha,
                beta,
                depth,
                reduction,
                extension,
                turn,
                ply,
                legal_moves_count,
            ) else {
                board.undo_move(&mv, captured, turn);
                self.history_stack.pop();
                return None;
            };

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
                if !is_capture {
                    let tried = quiet_moves_tried.get(..quiet_count).unwrap_or(&[]);
                    self.update_quiet_histories(depth, ply, mv, tried);
                }
                tt_flag = TTFlag::LowerBound;
                break;
            }
        }

        if legal_moves_count == 0 {
            self.history_stack.pop();
            if !in_check && has_repetition_move {
                return Some(0);
            }
            return Some(-self.calculate_mate_score(ply));
        }

        if excluded_move.is_none() {
            self.tt
                .store(hash, best_move_this_node, best_score, depth, tt_flag);
        }

        self.history_stack.pop();
        Some(best_score)
    }

    fn quiescence(
        &mut self,
        board: &mut Board,
        mut alpha: i32,
        beta: i32,
        turn: Color,
        ply: u8,
    ) -> i32 {
        if ply >= MAX_PLY {
            return self.eval_for_turn(board, turn);
        }

        self.nodes_searched += 1;

        let hash = board.zobrist_hash;
        if let Some(entry) = self.tt.probe(hash) {
            match entry.flag {
                TTFlag::Exact => return entry.score,
                TTFlag::LowerBound if entry.score >= beta => return entry.score,
                TTFlag::UpperBound if entry.score <= alpha => return entry.score,
                _ => {}
            }
        }

        let stand_pat = self.eval_for_turn(board, turn);

        if stand_pat >= beta {
            return beta;
        }
        if alpha < stand_pat {
            alpha = stand_pat;
        }

        let movegen = EngineMoveGen::new(&self.history_table);
        let captures = movegen.generate_captures(board, turn, &self.killer_moves);

        for mv in captures {
            let captured = board.get_piece(unsafe {
                BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
            });

            // Delta Pruning
            if let Some(cap_piece) = captured {
                let cap_val = get_piece_value(cap_piece.piece_type);
                if stand_pat + cap_val + 180 < alpha {
                    continue;
                }
            }

            board.apply_move(&mv, turn);

            if is_in_check(board, turn) || is_flying_general(board) {
                board.undo_move(&mv, captured, turn);
                continue;
            }

            let score =
                -self.quiescence(board, -beta, -alpha, turn.opposite(), ply.saturating_add(1));

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

    fn store_killer(&mut self, depth: u8, mv: Move) {
        if let Some([k0, k1]) = self.killer_moves.get_mut(depth as usize) {
            if *k0 != Some(mv) {
                *k1 = *k0;
                *k0 = Some(mv);
            }
        }
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
        board.refresh_nnue();
        let board = &mut board;
        let turn = game_state.turn;

        self.history_stack.clear();
        let initial_hash = Board::new().zobrist_hash;
        if !game_state.history.is_empty() {
            self.history_stack.push(initial_hash);
        }
        for record in &game_state.history {
            self.history_stack.push(record.hash);
        }

        let mut best_move = None;
        let mut final_depth = 0;
        let mut previous_score: Option<i32> = None;

        // History Aging
        for row in &mut self.history_table {
            for val in row.iter_mut() {
                *val /= 2;
            }
        }

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

                let movegen = EngineMoveGen::new(&self.history_table);
                let mut moves = movegen.generate_moves(board, turn, tt_move, &self.killer_moves, d);

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

                moves.retain(|mv| {
                    let captured = board.get_piece(unsafe {
                        BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
                    });
                    board.apply_move(mv, turn);
                    let legal = !is_in_check(board, turn) && !is_flying_general(board);
                    board.undo_move(mv, captured, turn);
                    legal
                });

                if moves.len() > 1 {
                    let non_repetition_moves: Vec<Move> = moves
                        .iter()
                        .filter(|mv| {
                            let captured = board.get_piece(unsafe {
                                BoardCoordinate::new_unchecked(
                                    mv.to_row as usize,
                                    mv.to_col as usize,
                                )
                            });
                            board.apply_move(mv, turn);
                            let next_hash = board.zobrist_hash;
                            let rep_count = self
                                .history_stack
                                .iter()
                                .filter(|&&h| h == next_hash)
                                .count();
                            board.undo_move(mv, captured, turn);
                            rep_count < 2
                        })
                        .copied()
                        .collect();

                    if !non_repetition_moves.is_empty() {
                        moves.retain(|mv| non_repetition_moves.contains(mv));
                    }
                }

                if moves.is_empty() {
                    break;
                }

                if self.check_time() {
                    break;
                }

                let mut time_out = false;

                for (moves_searched, mv) in moves.into_iter().enumerate() {
                    let captured = board.get_piece(unsafe {
                        BoardCoordinate::new_unchecked(mv.to_row as usize, mv.to_col as usize)
                    });
                    board.apply_move(&mv, turn);
                    if let Some(first_slot) = self.move_stack.first_mut() {
                        *first_slot = Some(mv);
                    }

                    let score_opt = if moves_searched == 0 {
                        self.alpha_beta(board, -beta, -alpha, d - 1, turn.opposite(), 1, None)
                    } else {
                        let mut val = self.alpha_beta(
                            board,
                            -alpha - 1,
                            -alpha,
                            d - 1,
                            turn.opposite(),
                            1,
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
                                    1,
                                    None,
                                );
                            }
                        }
                        val
                    };

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
                },
            )
        })
    }
}
