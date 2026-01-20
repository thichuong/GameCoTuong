use crate::logic::board::{Board, BoardCoordinate, Color, PieceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    OutOfBounds,
    NoPieceAtSource,
    NotYourTurn,
    InvalidMovePattern,
    BlockedPath,
    TargetOccupiedByFriendly,
    PalaceRestriction,
    RiverRestriction,
    SelfCheck,
    ThreeFoldRepetition,
}

/// Checks if a move is valid, including rule logic and self-check prevention.
pub fn is_valid_move(
    board: &Board,
    from: BoardCoordinate,
    to: BoardCoordinate,
    turn: Color,
) -> Result<(), MoveError> {
    // 1. Validate basic rules (geometry, path blocking, etc.)
    validate_piece_logic(board, from, to, turn)?;

    // 2. Simulate move to check for self-check
    let mut next_board = board.clone();
    // We know the move is valid geometrically, so we can just move the piece
    next_board.move_piece_quiet(from, to);

    if is_in_check(&next_board, turn) {
        return Err(MoveError::SelfCheck);
    }

    // 3. Check for Flying General (Generals cannot face each other)
    if is_flying_general(&next_board) {
        return Err(MoveError::SelfCheck); // Technically self-check
    }

    Ok(())
}

/// Checks if the `color` is currently in check.
use crate::logic::board::BitboardIterator;
use crate::logic::lookup::AttackTables;

/// Checks if the `color` is currently in check.
pub fn is_in_check(board: &Board, color: Color) -> bool {
    // Locate the General
    let general_idx = match color {
        Color::Red => 0,
        Color::Black => 7,
    };
    let general_bb = board.bitboards[general_idx];

    // Get general position (should be exactly one)
    let general_sq = if let Some(sq) = BitboardIterator::new(general_bb).next() {
        sq
    } else {
        return true; // No general means invalid state (or captured), treat as check
    };

    let (gr, gc) = Board::index_to_coord(general_sq);
    let enemy_color = color.opposite();
    let tables = AttackTables::get();

    // 1. Check Rook/Chariot Attacks
    let enemy_rooks = board.bitboards[match enemy_color {
        Color::Red => 4,
        Color::Black => 11,
    }];
    if enemy_rooks != 0 {
        let rank_occ = board.occupied_rows[gr];
        let file_occ = board.occupied_cols[gc];

        // Rank Check
        let rank_attacks = tables.get_rook_attacks(gc, rank_occ, 9);
        let er_rank = (enemy_rooks >> (gr * 9)) as u16 & 0x1FF;
        if (er_rank & rank_attacks) != 0 {
            return true;
        }

        // File Check
        let file_attacks = tables.get_rook_attacks(gr, file_occ, 10);
        let mut attacks = file_attacks;
        while attacks != 0 {
            let r = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            if (enemy_rooks & (1u128 << (r * 9 + gc))) != 0 {
                return true;
            }
        }
    }

    // 2. Check Cannon Attacks
    let enemy_cannons = board.bitboards[match enemy_color {
        Color::Red => 5,
        Color::Black => 12,
    }];
    if enemy_cannons != 0 {
        let rank_occ = board.occupied_rows[gr];
        let file_occ = board.occupied_cols[gc];

        let rank_attacks = tables.get_cannon_attacks(gc, rank_occ, 9);
        let er_rank = (enemy_cannons >> (gr * 9)) as u16 & 0x1FF;
        if (er_rank & rank_attacks) != 0 {
            return true;
        }

        let file_attacks = tables.get_cannon_attacks(gr, file_occ, 10);
        let mut attacks = file_attacks;
        while attacks != 0 {
            let r = attacks.trailing_zeros() as usize;
            attacks &= attacks - 1;
            if (enemy_cannons & (1u128 << (r * 9 + gc))) != 0 {
                return true;
            }
        }
    }

    // 3. Check Horse Attacks
    let enemy_horses = board.bitboards[match enemy_color {
        Color::Red => 3,
        Color::Black => 10,
    }];
    if enemy_horses != 0 {
        // Offsets: (dr, dc, leg_r_off, leg_c_off)
        // Offsets: (dr, dc, leg_r_off, leg_c_off)
        // Correct logic: leg is at (Start + dir). Start = Target - move.
        // For check, we look from General (Target) to Horse (Source).
        // Move is Source -> Target.
        // Leg is Source + 1 step orthogonal.
        // If relative to General, Source is at (dr, dc).
        // Leg is at (dr, dc) + 1 step towards General? No.
        // Leg is at Source + 1 step towards Target.
        // Source = (gr+dr, gc+dc). Target = (gr, gc).
        // If |dr|=2, step is row-wise towards Target. dr has sign S. Step is -S.
        // LegRow = (gr+dr) - sign(dr). LegCol = gc+dc.
        // LegRow relative to Gr: dr - sign(dr). LegCol relative to Gc: dc.
        // Example: dr=-2, dc=-1. sign(dr)=-1.
        // LegRelRow = -2 - (-1) = -1. LegRelCol = -1.
        // Wait, my previous manual calc was (-1, -1).
        // Yes.
        // So offsets should be (dr - sign(dr), dc)?
        // For |dr|=2: LegRel = (dr/2, dc).
        // For |dr|=1 (|dc|=2): LegRel = (dr, dc/2).
        // Let's re-verify.
        // Case 1: (-2, -1). LegRel (-1, -1).
        // Formula 1: (-1, -1). Correct.
        // Case 5: (-1, -2). LegRel (-1, -1).
        // Formula 2: (-1, -1). Correct.
        // So the table values should be:
        let offsets = [
            (-2, -1, -1, -1),
            (-2, 1, -1, 1),
            (2, -1, 1, -1),
            (2, 1, 1, 1),
            (-1, -2, -1, -1),
            (-1, 2, -1, 1),
            (1, -2, 1, -1),
            (1, 2, 1, 1),
        ];

        for (dr, dc, lr, lc) in offsets {
            let tr = gr as isize + dr;
            let tc = gc as isize + dc;

            if (0..10).contains(&tr) && (0..9).contains(&tc) {
                let check_sr = tr as usize;
                let check_sc = tc as usize;

                if (enemy_horses & (1u128 << (check_sr * 9 + check_sc))) != 0 {
                    // Found enemy horse, check leg
                    let leg_r = (gr as isize + lr) as usize;
                    let leg_c = (gc as isize + lc) as usize;
                    if (board.occupied & (1u128 << (leg_r * 9 + leg_c))) == 0 {
                        return true;
                    }
                }
            }
        }
    }

    // 4. Check Soldier Attacks
    let enemy_soldiers = board.bitboards[match enemy_color {
        Color::Red => 6,
        Color::Black => 13,
    }];
    if enemy_soldiers != 0 {
        // Red soldiers attack upwards (+1 row normally), but from perspective of General being attacked:
        // If we are Black Gen, Red Soldier below us (row - 1) attacks us.
        // If we are Red Gen, Black Soldier above us (row + 1) attacks us.
        let backward_check = if enemy_color == Color::Red { -1 } else { 1 };

        let fr = gr as isize + backward_check;
        if (0..10).contains(&fr) {
            if (enemy_soldiers & (1u128 << (fr as usize * 9 + gc))) != 0 {
                return true;
            }
        }

        // Side attacks
        for dc in [-1, 1] {
            let sc = gc as isize + dc;
            if (0..9).contains(&sc) {
                if (enemy_soldiers & (1u128 << (gr * 9 + sc as usize))) != 0 {
                    return true;
                }
            }
        }
    }

    false
}

pub fn is_flying_general(board: &Board) -> bool {
    let red_gen_bb = board.bitboards[0];
    let black_gen_bb = board.bitboards[7];

    let red_sq = if let Some(sq) = BitboardIterator::new(red_gen_bb).next() {
        sq
    } else {
        return false;
    };

    let black_sq = if let Some(sq) = BitboardIterator::new(black_gen_bb).next() {
        sq
    } else {
        return false;
    };

    let (r1, c1) = Board::index_to_coord(red_sq);
    let (r2, c2) = Board::index_to_coord(black_sq);

    if c1 != c2 {
        return false;
    }

    // Check for pieces in between
    let min_r = r1.min(r2);
    let max_r = r1.max(r2);

    if max_r > min_r + 1 {
        let mask = ((1u16 << max_r) - 1) ^ ((1u16 << (min_r + 1)) - 1);
        let col_occ = board.occupied_cols[c1];
        if (col_occ & mask) == 0 {
            return true;
        }
    } else {
        // Generals are adjacent on same file (no pieces between)
        return true;
    }

    false
}

/// Validates the geometry and specific rules for a piece move, IGNORING self-check.
fn validate_piece_logic(
    board: &Board,
    from: BoardCoordinate,
    to: BoardCoordinate,
    turn: Color,
) -> Result<(), MoveError> {
    // Basic bounds check implicit in BoardCoordinate type

    let piece = board.get_piece(from).ok_or(MoveError::NoPieceAtSource)?;

    if piece.color != turn {
        return Err(MoveError::NotYourTurn);
    }

    if from == to {
        return Err(MoveError::InvalidMovePattern);
    }

    if let Some(target) = board.get_piece(to) {
        if target.color == piece.color {
            return Err(MoveError::TargetOccupiedByFriendly);
        }
    }

    let d_row = to.row.abs_diff(from.row);
    let d_col = to.col.abs_diff(from.col);

    match piece.piece_type {
        PieceType::General => validate_general(piece.color, to, d_row, d_col),
        PieceType::Advisor => validate_advisor(piece.color, to, d_row, d_col),
        PieceType::Elephant => validate_elephant(board, piece.color, from, to, d_row, d_col),
        PieceType::Horse => validate_horse(board, from, to, d_row, d_col),
        PieceType::Chariot => validate_chariot(board, from, to, d_row, d_col),
        PieceType::Cannon => validate_cannon(board, from, to, d_row, d_col),
        PieceType::Soldier => validate_soldier(piece.color, from, to, d_row, d_col),
    }
}

fn validate_general(
    color: Color,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if d_row + d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }
    if !is_in_palace(color, to) {
        return Err(MoveError::PalaceRestriction);
    }
    Ok(())
}

fn validate_advisor(
    color: Color,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if d_row != 1 || d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }
    if !is_in_palace(color, to) {
        return Err(MoveError::PalaceRestriction);
    }
    Ok(())
}

fn validate_elephant(
    board: &Board,
    color: Color,
    from: BoardCoordinate,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if d_row != 2 || d_col != 2 {
        return Err(MoveError::InvalidMovePattern);
    }
    if is_crossing_river(color, to.row) {
        return Err(MoveError::RiverRestriction);
    }
    let eye_row = usize::midpoint(from.row, to.row);
    let eye_col = usize::midpoint(from.col, to.col);
    let eye_pos = unsafe { BoardCoordinate::new_unchecked(eye_row, eye_col) };
    if board.get_piece(eye_pos).is_some() {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_horse(
    board: &Board,
    from: BoardCoordinate,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if !((d_row == 2 && d_col == 1) || (d_row == 1 && d_col == 2)) {
        return Err(MoveError::InvalidMovePattern);
    }
    let leg_row = if d_row == 2 {
        usize::midpoint(from.row, to.row)
    } else {
        from.row
    };
    let leg_col = if d_col == 2 {
        usize::midpoint(from.col, to.col)
    } else {
        from.col
    };

    let leg_pos = unsafe { BoardCoordinate::new_unchecked(leg_row, leg_col) };
    if board.get_piece(leg_pos).is_some() {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_chariot(
    board: &Board,
    from: BoardCoordinate,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if d_row != 0 && d_col != 0 {
        return Err(MoveError::InvalidMovePattern);
    }
    if count_obstacles(board, from, to) > 0 {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_cannon(
    board: &Board,
    from: BoardCoordinate,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    if d_row != 0 && d_col != 0 {
        return Err(MoveError::InvalidMovePattern);
    }
    let obstacles = count_obstacles(board, from, to);
    let target = board.get_piece(to);

    if target.is_none() {
        if obstacles > 0 {
            return Err(MoveError::BlockedPath);
        }
    } else if obstacles != 1 {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_soldier(
    color: Color,
    from: BoardCoordinate,
    to: BoardCoordinate,
    d_row: usize,
    d_col: usize,
) -> Result<(), MoveError> {
    // 1. Check backward move
    if color == Color::Red {
        if to.row < from.row {
            return Err(MoveError::InvalidMovePattern);
        }
    } else if to.row > from.row {
        return Err(MoveError::InvalidMovePattern);
    }

    // 2. Check step size
    if d_row + d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }

    // 3. Check side move before river
    if !is_crossed_river(color, from.row) && d_col != 0 {
        return Err(MoveError::InvalidMovePattern);
    }

    Ok(())
}

fn is_in_palace(color: Color, pos: BoardCoordinate) -> bool {
    if !(3..=5).contains(&pos.col) {
        return false;
    }
    match color {
        Color::Red => pos.row <= 2,
        Color::Black => pos.row >= 7,
    }
}

const fn is_crossing_river(color: Color, row: usize) -> bool {
    match color {
        Color::Red => row > 4,
        Color::Black => row < 5,
    }
}

const fn is_crossed_river(color: Color, row: usize) -> bool {
    is_crossing_river(color, row)
}

fn count_obstacles(board: &Board, from: BoardCoordinate, to: BoardCoordinate) -> usize {
    let mut count = 0;
    if from.row == to.row {
        let (min, max) = if from.col < to.col {
            (from.col, to.col)
        } else {
            (to.col, from.col)
        };
        for c in (min + 1)..max {
            let pos = unsafe { BoardCoordinate::new_unchecked(from.row, c) };
            if board.get_piece(pos).is_some() {
                count += 1;
            }
        }
    } else {
        let (min, max) = if from.row < to.row {
            (from.row, to.row)
        } else {
            (to.row, from.row)
        };
        for r in (min + 1)..max {
            let pos = unsafe { BoardCoordinate::new_unchecked(r, from.col) };
            if board.get_piece(pos).is_some() {
                count += 1;
            }
        }
    }
    count
}
