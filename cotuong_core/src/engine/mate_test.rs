use crate::engine::config::EngineConfig;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::{Board, Color};
use crate::logic::game::GameState;
use std::sync::Arc;

#[test]
fn test_absolute_checkmate() {
    // Setup a "Mate in 1" position
    // Red Chariot at (9, 0)
    // Black General at (0, 4)
    // Red Chariot moves to (0, 0) -> Checkmate (if no blockers/defenders)

    // Let's use a FEN string if possible, or construct manually.
    // Since FEN parser might not be exposed or easy, I'll construct manually.
    // Actually, let's try to find a simple mate in 1.

    // Position:
    // Red General at 9, 4
    // Red Chariot at 1, 0
    // Black General at 0, 4
    // Move: Chariot 1,0 -> 0,0 is checkmate? No, General can take if no protection.
    // Let's use "Iron Bolt" mate (Chariot + General face-off)
    // Red General 9,4
    // Black General 0,4
    // Red Chariot at 1,4 (blocking face-off)
    // If Red Chariot moves, it reveals check.

    // Better:
    // Red Chariot at 9,0.
    // Black General at 0,4.
    // Red Cannon at 2,4.
    // Red Chariot moves to 0,0. Check.

    // Let's use a standard "Face-to-face" mate with Chariot.
    // Red General: 9, 4
    // Black General: 0, 4
    // Red Chariot: 5, 4
    // This is already checking? No, Chariot is at 5,4.

    // Let's use a known simple mate.
    // Red moves Chariot to bottom rank to mate.

    let mut board = Board::default();
    board.clear();

    // Red General at Bottom (0, 4)
    board.add_piece(0, 4, crate::logic::board::PieceType::General, Color::Red);
    // Black General at Top (9, 4)
    board.add_piece(9, 4, crate::logic::board::PieceType::General, Color::Black);

    // Red Chariot at (8, 0)
    board.add_piece(8, 0, crate::logic::board::PieceType::Chariot, Color::Red);

    // Block Black General from moving sideways?
    // Black General at (9,4).
    // Moves: (9,3), (9,5), (8,4).
    // Chariot at (8,0) moves to (9,0).
    // (9,0) attacks (9,4) horizontally.
    // Attacks (9,3), (9,5).
    // So sideways are covered.
    // What about (8,4)?
    // Chariot at (9,0) does NOT attack (8,4).
    // So Black General can escape to (8,4).
    // We need to block (8,4).
    // Put a Black Soldier at (8,4).
    board.add_piece(8, 4, crate::logic::board::PieceType::Soldier, Color::Black);

    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);

    let game_state = GameState {
        board,
        turn: Color::Red,
        ..Default::default()
    };

    let limit = SearchLimit::Depth(4);
    let (best_move, _) = engine.search(&game_state, limit, &[]).unwrap();
    println!("Best move: {:?}", best_move);

    // Expected move: (8,0) -> (9,0)
    assert_eq!(best_move.from_row, 8);
    assert_eq!(best_move.from_col, 0);
    assert_eq!(best_move.to_row, 9);
    assert_eq!(best_move.to_col, 0);

    // Also verify the score indicates mate
    // We can't easily check the score from here as `search` returns just the move and stats.
    // But if it picks the mate, it's good.
}
