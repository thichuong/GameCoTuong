use crate::logic::board::{Board, Color};
use rand::seq::SliceRandom;

pub fn get_book_move(board: &Board, turn: Color) -> Option<((usize, usize), (usize, usize))> {
    let fen = board.to_fen_string(turn);
    // Only match the piece placement and turn, ignore move counts if any
    // Our to_fen_string returns "pieces turn" e.g. "rnbakabnr/9/... w"

    let mut rng = rand::thread_rng();

    match fen.as_str() {
        // Starting Position (Red to move)
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w" => {
            let moves = vec![
                // Central Cannon (Pháo Đầu): C2.5
                ((2, 1), (2, 4)), // C8 -> C5 (in array coords: row 2, col 1 -> row 2, col 4) ??
                // Wait, array coords:
                // Red is at bottom (rows 0-4).
                // Red Cannons are at row 2, cols 1 and 7.
                // Move (2, 1) -> (2, 4) is Cannon to center.
                ((2, 7), (2, 4)), // Other Cannon to center
                // Elephant (Tiên Nhân Chỉ Lộ): E3.5 or E7.5
                // Elephants at row 0, cols 2 and 6.
                // Move (0, 2) -> (2, 4) ?? No, Elephant moves 2 diag.
                // Elephant moves to (2, 0) or (2, 4)? No.
                // Elephant moves: (0, 2) -> (2, 0) or (2, 4).
                // Wait, Elephant is at (0, 2). Valid moves: (2, 0), (2, 4).
                ((0, 2), (2, 4)),
                ((0, 6), (2, 4)),
                // Pawn 3 or 7 (P3.1, P7.1)
                // Pawns at row 3, cols 0, 2, 4, 6, 8.
                ((3, 2), (4, 2)),
                ((3, 6), (4, 6)),
            ];
            moves.choose(&mut rng).copied()
        }

        // Response to Central Cannon (Black to move)
        // Red played C2.5 (2,7)->(2,4) or (2,1)->(2,4)
        // FEN for (2,7)->(2,4): "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1C1P1P/1C7/9/RNBAKABNR b"
        // Wait, let's calculate row 2 correctly.
        // Orig: 1C5C1. (0:1, 1:C, 2..6:5, 7:C, 8:1)
        // Move (2,7)->(2,4).
        // New: 0:1, 1:C, 2:1, 3:1, 4:C, 5:1, 6:1, 7:1, 8:1.
        // Groups: 1, C, 2, C, 4. -> "1C2C4"
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1C1P1P/1C2C4/9/RNBAKABNR b" => {
            let moves = vec![
                // Screen Horse (Bình Phong Mã): H8+7 (9,7)->(7,6) or H2+3 (9,1)->(7,2)
                ((9, 7), (7, 6)),
                ((9, 1), (7, 2)),
                // Same Direction Cannon (Thuận Pháo): C8.5 (7,7)->(7,4)
                ((7, 7), (7, 4)),
            ];
            moves.choose(&mut rng).copied()
        }
        // Red played C8.5 (2,1)->(2,4)
        // Orig: 1C5C1. Move (2,1)->(2,4).
        // New: 0:1, 1:1, 2:1, 3:1, 4:C, 5:1, 6:1, 7:C, 8:1.
        // Groups: 4, C, 2, C, 1. -> "4C2C1"
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1C1P1P/4C2C1/9/RNBAKABNR b" => {
            let moves = vec![
                // Screen Horse
                ((9, 7), (7, 6)),
                ((9, 1), (7, 2)),
                // Same Direction Cannon: C2.5 (7,1)->(7,4)
                ((7, 1), (7, 4)),
            ];
            moves.choose(&mut rng).copied()
        }

        // Let's rely on the user's provided coordinates if possible, but I need to match the FEN.
        // Since I can't easily predict the FEN for every move without running the engine,
        // I will just add the starting position for now, and maybe one or two obvious ones if I can derive the FEN.

        // Actually, I can just print the FEN in the game log to debug and add more later.
        // For now, I'll implement the structure and the starting position.
        _ => None,
    }
}
