use crate::logic::board::{Board, Color, PieceType};

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
}

/// Checks if a move is valid, including rule logic and self-check prevention.
pub fn is_valid_move(
    board: &Board,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    turn: Color,
) -> Result<(), MoveError> {
    // 1. Validate basic rules (geometry, path blocking, etc.)
    validate_piece_logic(board, from_row, from_col, to_row, to_col, turn)?;

    // 2. Simulate move to check for self-check
    let mut next_board = board.clone();
    // We know the move is valid geometrically, so we can just move the piece
    // (We don't need to handle capture logic here other than overwriting)
    let piece = next_board.grid[from_row][from_col]
        .take()
        .ok_or(MoveError::NoPieceAtSource)?;
    next_board.grid[to_row][to_col] = Some(piece);

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
            if let Some(p) = board.get_piece(r, c) {
                if p.color == color && p.piece_type == PieceType::General {
                    general_pos = Some((r, c));
                    break;
                }
            }
        }
        if general_pos.is_some() {
            break;
        }
    }

    let (g_row, g_col) = match general_pos {
        Some(pos) => pos,
        None => return true, // Should not happen, but if no general, you are in trouble
    };

    // Check if any enemy piece can move to (g_row, g_col)
    let enemy_color = color.opposite();
    for r in 0..10 {
        for c in 0..9 {
            if let Some(p) = board.get_piece(r, c) {
                if p.color == enemy_color {
                    // Check if this piece can attack the general
                    // We ignore self-check for the attacker here, just geometry
                    if validate_piece_logic(board, r, c, g_row, g_col, enemy_color).is_ok() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn is_flying_general(board: &Board) -> bool {
    let mut red_gen = None;
    let mut black_gen = None;

    for r in 0..10 {
        for c in 3..6 {
            // Generals are only in cols 3-5
            if let Some(p) = board.get_piece(r, c) {
                if p.piece_type == PieceType::General {
                    if p.color == Color::Red {
                        red_gen = Some((r, c));
                    } else {
                        black_gen = Some((r, c));
                    }
                }
            }
        }
    }

    if let (Some((r1, c1)), Some((r2, c2))) = (red_gen, black_gen) {
        if c1 == c2 {
            // Check if there are pieces between them
            let min_r = r1.min(r2);
            let max_r = r1.max(r2);
            for r in (min_r + 1)..max_r {
                if board.get_piece(r, c1).is_some() {
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
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    turn: Color,
) -> Result<(), MoveError> {
    // Basic bounds check
    if from_row >= 10 || from_col >= 9 || to_row >= 10 || to_col >= 9 {
        return Err(MoveError::OutOfBounds);
    }

    let piece = board
        .get_piece(from_row, from_col)
        .ok_or(MoveError::NoPieceAtSource)?;

    if piece.color != turn {
        return Err(MoveError::NotYourTurn);
    }

    if from_row == to_row && from_col == to_col {
        return Err(MoveError::InvalidMovePattern);
    }

    if let Some(target) = board.get_piece(to_row, to_col) {
        if target.color == piece.color {
            return Err(MoveError::TargetOccupiedByFriendly);
        }
    }

    let d_row = (to_row as isize - from_row as isize).abs();
    let d_col = (to_col as isize - from_col as isize).abs();

    match piece.piece_type {
        PieceType::General => validate_general(
            piece.color,
            from_row,
            from_col,
            to_row,
            to_col,
            d_row,
            d_col,
        ),
        PieceType::Advisor => validate_advisor(piece.color, to_row, to_col, d_row, d_col),
        PieceType::Elephant => validate_elephant(
            board,
            piece.color,
            from_row,
            from_col,
            to_row,
            to_col,
            d_row,
            d_col,
        ),
        PieceType::Horse => validate_horse(board, from_row, from_col, to_row, to_col, d_row, d_col),
        PieceType::Chariot => {
            validate_chariot(board, from_row, from_col, to_row, to_col, d_row, d_col)
        }
        PieceType::Cannon => {
            validate_cannon(board, from_row, from_col, to_row, to_col, d_row, d_col)
        }
        PieceType::Soldier => validate_soldier(piece.color, from_row, to_row, to_col, d_row, d_col),
    }
}

// ... (Rest of the validation functions remain the same, I will include them for completeness)

fn validate_general(
    color: Color,
    _from_row: usize,
    _from_col: usize,
    to_row: usize,
    to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if d_row + d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }
    if !is_in_palace(color, to_row, to_col) {
        return Err(MoveError::PalaceRestriction);
    }
    Ok(())
}

fn validate_advisor(
    color: Color,
    to_row: usize,
    to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if d_row != 1 || d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }
    if !is_in_palace(color, to_row, to_col) {
        return Err(MoveError::PalaceRestriction);
    }
    Ok(())
}

fn validate_elephant(
    board: &Board,
    color: Color,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    _to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if d_row != 2 || d_col != 2 {
        return Err(MoveError::InvalidMovePattern);
    }
    if is_crossing_river(color, to_row) {
        return Err(MoveError::RiverRestriction);
    }
    let eye_row = (from_row + to_row) / 2;
    let eye_col = (from_col + _to_col) / 2;
    if board.get_piece(eye_row, eye_col).is_some() {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_horse(
    board: &Board,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if !((d_row == 2 && d_col == 1) || (d_row == 1 && d_col == 2)) {
        return Err(MoveError::InvalidMovePattern);
    }
    let leg_row = if d_row == 2 {
        (from_row + to_row) / 2
    } else {
        from_row
    };
    let leg_col = if d_col == 2 {
        (from_col + to_col) / 2
    } else {
        from_col
    };

    if board.get_piece(leg_row, leg_col).is_some() {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_chariot(
    board: &Board,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if d_row != 0 && d_col != 0 {
        return Err(MoveError::InvalidMovePattern);
    }
    if count_obstacles(board, from_row, from_col, to_row, to_col) > 0 {
        return Err(MoveError::BlockedPath);
    }
    Ok(())
}

fn validate_cannon(
    board: &Board,
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    if d_row != 0 && d_col != 0 {
        return Err(MoveError::InvalidMovePattern);
    }
    let obstacles = count_obstacles(board, from_row, from_col, to_row, to_col);
    let target = board.get_piece(to_row, to_col);

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
    from_row: usize,
    to_row: usize,
    _to_col: usize,
    d_row: isize,
    d_col: isize,
) -> Result<(), MoveError> {
    let forward = match color {
        Color::Red => 1,
        Color::Black => -1,
    };

    let row_diff = to_row as isize - from_row as isize;

    if (color == Color::Red && row_diff < 0) || (color == Color::Black && row_diff > 0) {
        return Err(MoveError::InvalidMovePattern);
    }

    if !is_crossed_river(color, from_row) {
        if d_col != 0 || row_diff != forward {
            return Err(MoveError::InvalidMovePattern);
        }
    } else if d_row + d_col != 1 {
        return Err(MoveError::InvalidMovePattern);
    }
    Ok(())
}

fn is_in_palace(color: Color, row: usize, col: usize) -> bool {
    if !(3..=5).contains(&col) {
        return false;
    }
    match color {
        Color::Red => row <= 2,
        Color::Black => row >= 7,
    }
}

fn is_crossing_river(color: Color, row: usize) -> bool {
    match color {
        Color::Red => row > 4,
        Color::Black => row < 5,
    }
}

fn is_crossed_river(color: Color, row: usize) -> bool {
    is_crossing_river(color, row)
}

fn count_obstacles(board: &Board, r1: usize, c1: usize, r2: usize, c2: usize) -> usize {
    let mut count = 0;
    if r1 == r2 {
        let (min, max) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        for c in (min + 1)..max {
            if board.get_piece(r1, c).is_some() {
                count += 1;
            }
        }
    } else {
        let (min, max) = if r1 < r2 { (r1, r2) } else { (r2, r1) };
        for r in (min + 1)..max {
            if board.get_piece(r, c1).is_some() {
                count += 1;
            }
        }
    }
    count
}
