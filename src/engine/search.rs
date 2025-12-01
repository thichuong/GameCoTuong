use crate::engine::config::EngineConfig;
use crate::engine::eval::SimpleEvaluator;
use crate::engine::move_list::MoveList;
use crate::engine::zobrist::{TTFlag, TranspositionTable};
use crate::engine::{Evaluator, Move, SearchLimit, SearchStats, Searcher};
use crate::logic::board::{Board, Color, PieceType};
use crate::logic::game::GameState;
use crate::logic::rules::{is_flying_general, is_in_check};
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

    #[allow(clippy::too_many_lines)]
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

        // Null Move Pruning
        // Conditions:
        // 1. Depth >= 3 (avoid pruning at low depths)
        // 2. Not in check (null move is illegal in check)
        // 3. Not a mate score (beta < 15000)
        // 4. Not in PV? (We don't track PV explicitly here yet, but beta-alpha > 1 usually implies PV)
        //    For simplicity, we just do it if depth >= 3.
        if depth >= 3 && beta.abs() < 15000 && !crate::logic::rules::is_in_check(board, turn) {
            let r = 2; // Reduction
            let mut next_board = board.clone();
            next_board.apply_null_move();

            // Null window search with reduced depth
            // We pass -beta, -beta+1 because we want to prove that null move is >= beta (fail high)
            if let Some(score) = self.alpha_beta(
                &next_board,
                depth - 1 - r,
                -beta,
                -beta + 1,
                turn.opposite(),
            ) {
                if -score >= beta {
                    self.history_stack.pop();
                    return Some(beta); // Cutoff
                }
            } else {
                self.history_stack.pop();
                return None; // Time out
            }
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

        // Dynamic Move Limiting (Forward Pruning)
        // Method 0: Dynamic Limiting
        // Method 2: Both
        if self.config.pruning_method == 0 || self.config.pruning_method == 2 {
            // Formula: keep_count = base + (depth^2 * multiplier)
            // Base = 8
            let d = f32::from(depth);
            let multiplier = self.config.pruning_multiplier;
            // Safety: depth is u8 (max 255), multiplier is f32 (max 2.0).
            // 8 + 255^2 * 2.0 ≈ 130,000, which fits easily in usize.
            // All inputs are positive, so no sign loss.
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let limit = (8.0 + d * d * multiplier) as usize;
            let limit = limit.min(moves.len());
            if moves.len() > limit {
                moves.truncate(limit);
            }
        }

        let mut best_score = -30000;
        let mut best_move_this_node = None; // Renamed to avoid conflict with `best_move` from TT
        let alpha_orig = alpha;

        for (moves_searched, mv) in moves.into_iter().enumerate() {
            let mut next_board = board.clone();
            next_board.apply_move(&mv, turn);

            let mut score;

            // LMR Logic
            let mut reduction = 0;
            if depth >= 3
                && moves_searched >= 4
                && (self.config.pruning_method == 1 || self.config.pruning_method == 2)
            {
                let is_capture = board.get_piece(mv.to_row, mv.to_col).is_some();
                // Also check if it gives check? (Expensive to check here, maybe skip for now)
                if !is_capture {
                    // Formula: reduction = 1 + ln(depth) * ln(moves_searched) / 2
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    {
                        let r = 1.0
                            + (f64::from(depth).ln() * f64::from(moves_searched as u32).ln()) / 2.0;
                        reduction = (r as u8).min(depth - 1);
                    }
                }
            }

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
                // Late moves: Null window search (PVS) with LMR
                // Try with reduced depth first
                let search_depth = depth - 1 - reduction;

                // Ensure we don't drop below 0 (handled by min(depth-1) above, but safe check)
                // if search_depth == 0 { search_depth = 1; } // This caused infinite recursion at depth 1!

                let mut val = self.alpha_beta(
                    &next_board,
                    search_depth,
                    -alpha - 1,
                    -alpha,
                    turn.opposite(),
                );

                // If we reduced and it failed high (score > alpha), re-search with full depth (null window)
                if let Some(v) = val {
                    if -v > alpha && reduction > 0 {
                        val = self.alpha_beta(
                            &next_board,
                            depth - 1,
                            -alpha - 1,
                            -alpha,
                            turn.opposite(),
                        );
                    }
                }

                match val {
                    None => {
                        self.history_stack.pop();
                        return None;
                    }
                    Some(v) => score = -v,
                }

                if score > alpha && score < beta {
                    // Fail high in null window, re-search with full window
                    let val =
                        self.alpha_beta(&next_board, depth - 1, -beta, -alpha, turn.opposite());
                    match val {
                        None => {
                            self.history_stack.pop();
                            return None;
                        }
                        Some(v) => score = -v,
                    }
                }
            }

            if score > best_score {
                best_score = score;
                best_move_this_node = Some(mv);
            }
            if score > alpha {
                alpha = score;
            }

            // moves_searched is now updated by enumerate

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
        self.generate_moves_internal(board, turn, best_move, depth, false)
    }

    fn generate_captures(&self, board: &Board, turn: Color) -> MoveList {
        self.generate_moves_internal(board, turn, None, 0, true)
    }

    fn generate_moves_internal(
        &self,
        board: &Board,
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

        for r in 0..10 {
            for c in 0..9 {
                if let Some(p) = board.get_piece(r, c) {
                    if p.color == turn {
                        match p.piece_type {
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
            }
        }
        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }

    #[allow(clippy::too_many_arguments)]
    fn add_move(
        &self,
        board: &Board,
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

        let target = board.get_piece(tr, tc);
        if let Some(t) = target {
            if t.color == turn {
                return; // Blocked by friendly
            }
        }

        if only_captures && target.is_none() {
            return;
        }

        // Legality Check (Self-check & Flying General)
        // We must clone to check. This is the cost we pay, but we do it far less often now.
        // Optimization: We could implement a lighter check that doesn't full clone,
        // but for now, let's rely on the reduction in candidate moves.
        let mut next_board = board.clone();
        // Manually apply move to avoid overhead of Board::apply_move (hashing etc not needed for check)
        // Actually, is_in_check doesn't need hash.
        // But we need to move the piece.
        // Let's just use a simplified move application on the grid.
        next_board.grid[tr][tc] = next_board.grid[r][c];
        next_board.grid[r][c] = None;

        if is_in_check(&next_board, turn) || is_flying_general(&next_board) {
            return;
        }

        // Scoring
        let mut score;
        let is_hash_move = if let Some(bm) = best_move {
            bm.from_row == r && bm.from_col == c && bm.to_row == tr && bm.to_col == tc
        } else {
            false
        };

        if is_hash_move {
            score = self.config.score_hash_move;
        } else if let Some(t) = target {
            // MVV-LVA
            let victim_val = self.get_piece_value(t.piece_type);
            let attacker_val = self.get_piece_value(board.get_piece(r, c).unwrap().piece_type);
            score = self.config.score_capture_base + victim_val - (attacker_val / 10);
        } else {
            let is_killer_move = killers.iter().any(|k| {
                if let Some(km) = k {
                    km.from_row == r && km.from_col == c && km.to_row == tr && km.to_col == tc
                } else {
                    false
                }
            });

            if is_killer_move {
                score = self.config.score_killer_move;
            } else {
                // History
                let from_idx = r * 9 + c;
                let to_idx = tr * 9 + tc;
                #[allow(clippy::indexing_slicing)]
                {
                    score = self.history_table[from_idx][to_idx];
                }
                if score > self.config.score_history_max {
                    score = self.config.score_history_max;
                }
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

    #[allow(clippy::too_many_arguments)]
    fn gen_rook_moves(
        &self,
        board: &Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dr, dc) in dirs {
            for i in 1..10 {
                let tr = (r as i32 + dr * i) as usize;
                let tc = (c as i32 + dc * i) as usize;
                if tr >= 10 || tc >= 9 {
                    break;
                }

                self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
                if board.get_piece(tr, tc).is_some() {
                    break;
                } // Blocked
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_cannon_moves(
        &self,
        board: &Board,
        turn: Color,
        r: usize,
        c: usize,
        moves: &mut MoveList,
        bm: Option<Move>,
        k: &[Option<Move>; 2],
        oc: bool,
    ) {
        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (dr, dc) in dirs {
            let mut jumped = false;
            for i in 1..10 {
                let tr = (r as i32 + dr * i) as usize;
                let tc = (c as i32 + dc * i) as usize;
                if tr >= 10 || tc >= 9 {
                    break;
                }

                if let Some(_) = board.get_piece(tr, tc) {
                    if !jumped {
                        jumped = true;
                        continue;
                    } else {
                        // Second piece (capture target)
                        self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
                        break; // Cannot jump over two
                    }
                }

                if !jumped {
                    self.add_move(board, turn, (r, c), (tr, tc), moves, bm, k, oc);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_horse_moves(
        &self,
        board: &Board,
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
        // Corresponding blocking legs
        // If move is (-2, -1) or (-2, 1), leg is (-1, 0)
        // If move is (2, -1) or (2, 1), leg is (1, 0)
        // If move is (-1, -2) or (1, -2), leg is (0, -1)
        // If move is (-1, 2) or (1, 2), leg is (0, 1)

        for (dr, dc) in moves_offsets {
            let tr = r as i32 + dr;
            let tc = c as i32 + dc;
            if tr < 0 || tr >= 10 || tc < 0 || tc >= 9 {
                continue;
            }

            // Check leg
            let leg_r = r as i32 + if dr.abs() == 2 { dr / 2 } else { 0 };
            let leg_c = c as i32 + if dc.abs() == 2 { dc / 2 } else { 0 };

            if board.get_piece(leg_r as usize, leg_c as usize).is_none() {
                self.add_move(
                    board,
                    turn,
                    (r, c),
                    (tr as usize, tc as usize),
                    moves,
                    bm,
                    k,
                    oc,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_elephant_moves(
        &self,
        board: &Board,
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
            let tr = r as i32 + dr;
            let tc = c as i32 + dc;
            if tr < 0 || tr >= 10 || tc < 0 || tc >= 9 {
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
            let eye_r = r as i32 + dr / 2;
            let eye_c = c as i32 + dc / 2;
            if board.get_piece(eye_r as usize, eye_c as usize).is_none() {
                self.add_move(
                    board,
                    turn,
                    (r, c),
                    (tr as usize, tc as usize),
                    moves,
                    bm,
                    k,
                    oc,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_advisor_moves(
        &self,
        board: &Board,
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
            let tr = r as i32 + dr;
            let tc = c as i32 + dc;
            if tr < 0 || tr >= 10 || tc < 0 || tc >= 9 {
                continue;
            }

            // Palace check
            if tc < 3 || tc > 5 {
                continue;
            }
            if turn == Color::Red && tr > 2 {
                continue;
            }
            if turn == Color::Black && tr < 7 {
                continue;
            }

            self.add_move(
                board,
                turn,
                (r, c),
                (tr as usize, tc as usize),
                moves,
                bm,
                k,
                oc,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_king_moves(
        &self,
        board: &Board,
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
            let tr = r as i32 + dr;
            let tc = c as i32 + dc;
            if tr < 0 || tr >= 10 || tc < 0 || tc >= 9 {
                continue;
            }

            // Palace check
            if tc < 3 || tc > 5 {
                continue;
            }
            if turn == Color::Red && tr > 2 {
                continue;
            }
            if turn == Color::Black && tr < 7 {
                continue;
            }

            self.add_move(
                board,
                turn,
                (r, c),
                (tr as usize, tc as usize),
                moves,
                bm,
                k,
                oc,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn gen_pawn_moves(
        &self,
        board: &Board,
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
        let tr = r as i32 + forward;
        if tr >= 0 && tr < 10 {
            self.add_move(board, turn, (r, c), (tr as usize, c), moves, bm, k, oc);
        }

        // Horizontal (if crossed river)
        let crossed_river = if turn == Color::Red { r > 4 } else { r < 5 };
        if crossed_river {
            for dc in [-1, 1] {
                let tc = c as i32 + dc;
                if tc >= 0 && tc < 9 {
                    self.add_move(board, turn, (r, c), (r, tc as usize), moves, bm, k, oc);
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
