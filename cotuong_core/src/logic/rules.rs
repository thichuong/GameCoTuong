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
pub fn is_in_check(board: &Board, color: Color) -> bool {
    // Find the General
    let mut general_pos = None;
    for r in 0..10 {
        for c in 0..9 {
            let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
            if let Some(p) = board.get_piece(pos) {
                if p.color == color && p.piece_type == PieceType::General {
                    general_pos = Some(pos);
                    break;
                }
            }
        }
        if general_pos.is_some() {
            break;
        }
    }

    let Some(g_pos) = general_pos else {
        return true;
    };

    // Check if any enemy piece can move to g_pos
    let enemy_color = color.opposite();
    for r in 0..10 {
        for c in 0..9 {
            let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
            if let Some(p) = board.get_piece(pos) {
                if p.color == enemy_color {
                    // Check if this piece can attack the general
                    // We ignore self-check for the attacker here, just geometry
                    if validate_piece_logic(board, pos, g_pos, enemy_color).is_ok() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub fn is_flying_general(board: &Board) -> bool {
    let mut red_gen = None;
    let mut black_gen = None;

    for r in 0..10 {
        for c in 3..6 {
            let pos = unsafe { BoardCoordinate::new_unchecked(r, c) };
            // Generals are only in cols 3-5
            if let Some(p) = board.get_piece(pos) {
                if p.piece_type == PieceType::General {
                    if p.color == Color::Red {
                        red_gen = Some(pos);
                    } else {
                        black_gen = Some(pos);
                    }
                }
            }
        }
    }

    if let (Some(p1), Some(p2)) = (red_gen, black_gen) {
        if p1.col == p2.col {
            // Check if there are pieces between them
            let min_r = p1.row.min(p2.row);
            let max_r = p1.row.max(p2.row);
            for r in (min_r + 1)..max_r {
                let check_pos = unsafe { BoardCoordinate::new_unchecked(r, p1.col) };
                if board.get_piece(check_pos).is_some() {
                    return false; // Blocked
                }
            }
            return true; // Flying General!
        }
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
