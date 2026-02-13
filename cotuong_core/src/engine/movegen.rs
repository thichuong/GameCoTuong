use crate::engine::config::EngineConfig;
use crate::engine::move_list::MoveList;
use crate::engine::Move;
use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};
use crate::logic::lookup::AttackTables;

pub struct MoveGenContext<'a> {
    pub board: &'a Board,
    pub turn: Color,
    pub moves: &'a mut MoveList,
    pub best_move: Option<Move>,
    pub killers: &'a [Option<Move>; 2],
    pub only_captures: bool,
    pub enemy_bb: u128,
}

pub struct EngineMoveGen<'a> {
    config: &'a EngineConfig,
    history_table: &'a [[i32; 90]],
}

impl<'a> EngineMoveGen<'a> {
    pub fn new(config: &'a EngineConfig, history_table: &'a [[i32; 90]]) -> Self {
        Self {
            config,
            history_table,
        }
    }

    pub fn generate_moves(
        &self,
        board: &mut Board,
        turn: Color,
        best_move: Option<Move>,
        killer_moves: &[[Option<Move>; 2]; 64],
        depth: u8,
    ) -> MoveList {
        self.generate_moves_internal(board, turn, best_move, killer_moves, depth, false)
    }

    pub fn generate_captures(
        &self,
        board: &mut Board,
        turn: Color,
        killer_moves: &[[Option<Move>; 2]; 64],
    ) -> MoveList {
        self.generate_moves_internal(board, turn, None, killer_moves, 0, true)
    }

    fn generate_moves_internal(
        &self,
        board: &Board,
        turn: Color,
        best_move: Option<Move>,
        killer_moves: &[[Option<Move>; 2]; 64],
        depth: u8,
        only_captures: bool,
    ) -> MoveList {
        let mut moves = MoveList::new();
        let killers = if (depth as usize) < killer_moves.len() {
            killer_moves.get(depth as usize).unwrap_or(&[None; 2])
        } else {
            &[None; 2]
        };

        let enemy_bb = board.get_color_bb(turn.opposite());

        let mut ctx = MoveGenContext {
            board,
            turn,
            moves: &mut moves,
            best_move,
            killers,
            only_captures,
            enemy_bb,
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
                    PieceType::Chariot => self.gen_rook_moves(&mut ctx, r, c),
                    PieceType::Cannon => self.gen_cannon_moves(&mut ctx, r, c),
                    PieceType::Horse => self.gen_horse_moves(&mut ctx, r, c),
                    PieceType::Elephant => self.gen_elephant_moves(&mut ctx, r, c),
                    PieceType::Advisor => self.gen_advisor_moves(&mut ctx, r, c),
                    PieceType::General => self.gen_king_moves(&mut ctx, r, c),
                    PieceType::Soldier => self.gen_pawn_moves(&mut ctx, r, c),
                }
            }
        }

        moves.sort_by(|a, b| b.score.cmp(&a.score));
        moves
    }

    fn add_move(&self, ctx: &mut MoveGenContext, from: (usize, usize), to: (usize, usize)) {
        let (r, c) = from;
        let (tr, tc) = to;

        let target_sq = Board::square_index(tr, tc);
        let is_occupied = (ctx.board.occupied & (1 << target_sq)) != 0;

        if is_occupied {
            if (ctx.board.get_color_bb(ctx.turn) & (1 << target_sq)) != 0 {
                return; // Blocked by friendly
            }
        } else if ctx.only_captures {
            return;
        }

        let target = if is_occupied {
            ctx.board
                .get_piece(unsafe { BoardCoordinate::new_unchecked(tr, tc) })
        } else {
            None
        };

        let mut score;
        let is_hash_move = ctx.best_move.is_some_and(|bm| {
            bm.from_row as usize == r
                && bm.from_col as usize == c
                && bm.to_row as usize == tr
                && bm.to_col as usize == tc
        });

        if is_hash_move {
            score = self.config.score_hash_move;
        } else if let Some(t) = target {
            let victim_val = self.get_piece_value(t.piece_type);
            let attacker_val = ctx
                .board
                .get_piece(unsafe { BoardCoordinate::new_unchecked(r, c) })
                .map_or(0, |p| self.get_piece_value(p.piece_type));
            score = self.config.score_capture_base + victim_val - (attacker_val / 10);
        } else {
            let is_killer_move = ctx.killers.iter().any(|k| {
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

        ctx.moves.push(Move {
            from_row: r as u8,
            from_col: c as u8,
            to_row: tr as u8,
            to_col: tc as u8,
            score,
        });
    }

    fn gen_rook_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let rank_occ = ctx.board.occupied_rows[r];
        let rank_attacks = tables.get_rook_attacks(c, rank_occ, 9);
        let mut attacks = rank_attacks;

        if ctx.only_captures {
            let enemy_rank = (ctx.enemy_bb >> (r * 9)) as u16 & 0x1FF;
            attacks &= enemy_rank;
        }

        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(ctx, (r, c), (r, col));
        }

        let file_occ = ctx.board.occupied_cols[c];
        let file_attacks = tables.get_rook_attacks(r, file_occ, 10);
        let mut attacks = file_attacks;

        if ctx.only_captures {
            while attacks != 0 {
                let row = attacks.trailing_zeros() as usize;
                attacks &= attacks - 1;
                if (ctx.enemy_bb & (1u128 << (row * 9 + c))) != 0 {
                    self.add_move(ctx, (r, c), (row, c));
                }
            }
        } else {
            while attacks != 0 {
                let row = attacks.trailing_zeros() as usize;
                attacks &= attacks - 1;
                self.add_move(ctx, (r, c), (row, c));
            }
        }
    }

    fn gen_cannon_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let rank_occ = ctx.board.occupied_rows[r];
        let rank_attacks = tables.get_cannon_attacks(c, rank_occ, 9);
        let mut attacks = rank_attacks;

        if ctx.only_captures {
            let enemy_rank = (ctx.enemy_bb >> (r * 9)) as u16 & 0x1FF;
            attacks &= enemy_rank;
        }

        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(ctx, (r, c), (r, col));
        }

        let file_occ = ctx.board.occupied_cols[c];
        let file_attacks = tables.get_cannon_attacks(r, file_occ, 10);
        let mut attacks = file_attacks;

        if ctx.only_captures {
            while attacks != 0 {
                let row = attacks.trailing_zeros() as usize;
                attacks &= attacks - 1;
                if (ctx.enemy_bb & (1u128 << (row * 9 + c))) != 0 {
                    self.add_move(ctx, (r, c), (row, c));
                }
            }
        } else {
            while attacks != 0 {
                let row = attacks.trailing_zeros() as usize;
                attacks &= attacks - 1;
                self.add_move(ctx, (r, c), (row, c));
            }
        }
    }

    fn gen_horse_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        for &(target_sq, leg_sq) in &tables.horse_moves[sq] {
            if (ctx.board.occupied & (1 << leg_sq)) == 0 {
                let (tr, tc) = Board::index_to_coord(target_sq);
                self.add_move(ctx, (r, c), (tr, tc));
            }
        }
    }

    fn gen_elephant_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        for &(target_sq, eye_sq) in &tables.elephant_moves[sq] {
            if (ctx.board.occupied & (1 << eye_sq)) == 0 {
                let (tr, tc) = Board::index_to_coord(target_sq);
                self.add_move(ctx, (r, c), (tr, tc));
            }
        }
    }

    fn gen_advisor_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        for &target_sq in &tables.advisor_moves[sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            self.add_move(ctx, (r, c), (tr, tc));
        }
    }

    fn gen_king_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        for &target_sq in &tables.general_moves[sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            self.add_move(ctx, (r, c), (tr, tc));
        }
    }

    fn gen_pawn_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;
        let color_idx = ctx.turn.index();

        for &target_sq in &tables.soldier_moves[color_idx][sq] {
            let (tr, tc) = Board::index_to_coord(target_sq);
            self.add_move(ctx, (r, c), (tr, tc));
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
