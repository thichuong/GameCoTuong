use crate::engine::move_list::MoveList;
use crate::engine::Move;
use crate::logic::board::{Board, Color, PieceType};
use crate::logic::eval_constants::get_piece_value;
use crate::logic::lookup::AttackTables;

pub const SCORE_HASH_MOVE: i32 = 1_000_000;
pub const SCORE_CAPTURE_BASE: i32 = 100_000;
pub const SCORE_KILLER_1: i32 = 90_000;
pub const SCORE_COUNTERMOVE: i32 = 85_000;
pub const SCORE_KILLER_2: i32 = 80_000;

pub struct MoveGenContext<'a> {
    pub board: &'a Board,
    pub turn: Color,
    pub moves: &'a mut MoveList,
    pub best_move: Option<Move>,
    pub killer_1: Option<Move>,
    pub killer_2: Option<Move>,
    pub countermove: Option<Move>,
    pub history: Option<&'a [[i32; 90]; 90]>,
    pub only_captures: bool,
    pub enemy_bb: u128,
    pub friendly_bb: u128,
    pub attacker_val: i32,
}

#[derive(Default, Clone, Copy)]
pub struct EngineMoveGen;

impl EngineMoveGen {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[inline]
    pub fn generate_moves(&self, board: &Board, turn: Color, best_move: Option<Move>) -> MoveList {
        self.generate_moves_internal(board, turn, best_move, &[], None, None, false)
    }

    #[inline]
    pub fn generate_moves_with_ordering(
        &self,
        board: &Board,
        turn: Color,
        best_move: Option<Move>,
        killers: &[Option<Move>],
        countermove: Option<Move>,
        history: Option<&[[i32; 90]; 90]>,
    ) -> MoveList {
        self.generate_moves_internal(board, turn, best_move, killers, countermove, history, false)
    }

    #[inline]
    pub fn generate_captures(&self, board: &Board, turn: Color) -> MoveList {
        self.generate_moves_internal(board, turn, None, &[], None, None, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn generate_moves_internal(
        &self,
        board: &Board,
        turn: Color,
        best_move: Option<Move>,
        killers: &[Option<Move>],
        countermove: Option<Move>,
        history: Option<&[[i32; 90]; 90]>,
        only_captures: bool,
    ) -> MoveList {
        let mut moves = MoveList::new();
        let enemy_bb = board.get_color_bb(turn.opposite());
        let friendly_bb = board.get_color_bb(turn);

        let killer_1 = killers.first().copied().flatten();
        let killer_2 = killers.get(1).copied().flatten();

        let mut ctx = MoveGenContext {
            board,
            turn,
            moves: &mut moves,
            best_move,
            killer_1,
            killer_2,
            countermove,
            history,
            only_captures,
            enemy_bb,
            friendly_bb,
            attacker_val: 0,
        };

        use crate::logic::board::BitboardIterator;

        let start = turn.index() * 7;
        let priority_pieces = [
            PieceType::Chariot,
            PieceType::Cannon,
            PieceType::Horse,
            PieceType::Soldier,
            PieceType::Elephant,
            PieceType::Advisor,
            PieceType::General,
        ];

        for &piece_type in &priority_pieces {
            let pt_idx = piece_type.index();
            let Some(&bb) = board.bitboards.get(start + pt_idx) else {
                continue;
            };
            ctx.attacker_val = get_piece_value(piece_type);

            for sq in BitboardIterator::new(bb) {
                let (r, c) = Board::index_to_coord(sq);

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

        moves
    }

    fn add_move(&self, ctx: &mut MoveGenContext, from: (usize, usize), to: (usize, usize)) {
        let (r, c) = from;
        let (tr, tc) = to;

        let target_sq = Board::square_index(tr, tc);
        let is_occupied = (ctx.board.occupied & (1 << target_sq)) != 0;

        if is_occupied {
            if (ctx.friendly_bb & (1 << target_sq)) != 0 {
                return; // Blocked by friendly
            }
        } else if ctx.only_captures {
            return;
        }

        let target = if is_occupied {
            ctx.board.grid.get(target_sq).copied().flatten()
        } else {
            None
        };

        let is_hash_move = ctx.best_move.is_some_and(|bm| bm.matches(r, c, tr, tc));
        let is_killer_1 = ctx.killer_1.is_some_and(|km| km.matches(r, c, tr, tc));
        let is_killer_2 = ctx.killer_2.is_some_and(|km| km.matches(r, c, tr, tc));
        let is_countermove = ctx.countermove.is_some_and(|cm| cm.matches(r, c, tr, tc));

        let score = if is_hash_move {
            SCORE_HASH_MOVE
        } else if let Some(t) = target {
            let victim_val = get_piece_value(t.piece_type);
            // MVV-LVA: Most Valuable Victim - Least Valuable Attacker
            SCORE_CAPTURE_BASE + victim_val * 10 - ctx.attacker_val
        } else if is_killer_1 {
            SCORE_KILLER_1
        } else if is_countermove {
            SCORE_COUNTERMOVE
        } else if is_killer_2 {
            SCORE_KILLER_2
        } else if let Some(hist) = ctx.history {
            let from_sq = Board::square_index(r, c);
            hist.get(from_sq)
                .and_then(|row| row.get(target_sq))
                .copied()
                .unwrap_or(0)
                .clamp(0, 50_000)
        } else {
            0
        };

        #[allow(clippy::cast_possible_truncation)]
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
        let rank_occ = ctx.board.occupied_rows.get(r).copied().unwrap_or(0);
        let rank_attacks = tables.get_rook_attacks(c, rank_occ, 9);
        let mut attacks = rank_attacks;

        if ctx.only_captures {
            #[allow(clippy::cast_possible_truncation)]
            let enemy_rank = (ctx.enemy_bb >> (r * 9)) as u16 & 0x1FF;
            attacks &= enemy_rank;
        }

        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(ctx, (r, c), (r, col));
        }

        let file_occ = ctx.board.occupied_cols.get(c).copied().unwrap_or(0);
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
        let rank_occ = ctx.board.occupied_rows.get(r).copied().unwrap_or(0);
        let rank_attacks = tables.get_cannon_attacks(c, rank_occ, 9);
        let mut attacks = rank_attacks;

        if ctx.only_captures {
            #[allow(clippy::cast_possible_truncation)]
            let enemy_rank = (ctx.enemy_bb >> (r * 9)) as u16 & 0x1FF;
            attacks &= enemy_rank;
        }

        while attacks != 0 {
            let col = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            self.add_move(ctx, (r, c), (r, col));
        }

        let file_occ = ctx.board.occupied_cols.get(c).copied().unwrap_or(0);
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

        if let Some(horse_list) = tables.horse_moves.get(sq) {
            for &(target_sq, leg_sq) in horse_list {
                if (ctx.board.occupied & (1 << leg_sq)) == 0 {
                    let (tr, tc) = Board::index_to_coord(target_sq);
                    self.add_move(ctx, (r, c), (tr, tc));
                }
            }
        }
    }

    fn gen_elephant_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        if let Some(elephant_list) = tables.elephant_moves.get(sq) {
            for &(target_sq, eye_sq) in elephant_list {
                if (ctx.board.occupied & (1 << eye_sq)) == 0 {
                    let (tr, tc) = Board::index_to_coord(target_sq);
                    self.add_move(ctx, (r, c), (tr, tc));
                }
            }
        }
    }

    fn gen_advisor_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        if let Some(advisor_list) = tables.advisor_moves.get(sq) {
            for &target_sq in advisor_list {
                let (tr, tc) = Board::index_to_coord(target_sq);
                self.add_move(ctx, (r, c), (tr, tc));
            }
        }
    }

    fn gen_king_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;

        if let Some(general_list) = tables.general_moves.get(sq) {
            for &target_sq in general_list {
                let (tr, tc) = Board::index_to_coord(target_sq);
                self.add_move(ctx, (r, c), (tr, tc));
            }
        }
    }

    fn gen_pawn_moves(&self, ctx: &mut MoveGenContext, r: usize, c: usize) {
        let tables = AttackTables::get();
        let sq = r * 9 + c;
        let color_idx = ctx.turn.index();

        if let Some(soldier_list) = tables
            .soldier_moves
            .get(color_idx)
            .and_then(|arr| arr.get(sq))
        {
            for &target_sq in soldier_list {
                let (tr, tc) = Board::index_to_coord(target_sq);
                self.add_move(ctx, (r, c), (tr, tc));
            }
        }
    }
}
