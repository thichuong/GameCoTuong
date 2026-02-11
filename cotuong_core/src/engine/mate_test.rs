#[cfg(test)]
mod tests {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use crate::logic::board::{Board, BoardCoordinate, Color};
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
        board.add_piece(
            BoardCoordinate::new(0, 4).unwrap(),
            crate::logic::board::PieceType::General,
            Color::Red,
        );
        // Black General at Top (9, 4)
        board.add_piece(
            BoardCoordinate::new(9, 4).unwrap(),
            crate::logic::board::PieceType::General,
            Color::Black,
        );

        // Red Chariot at (8, 0)
        board.add_piece(
            BoardCoordinate::new(8, 0).unwrap(),
            crate::logic::board::PieceType::Chariot,
            Color::Red,
        );

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
        board.add_piece(
            BoardCoordinate::new(8, 4).unwrap(),
            crate::logic::board::PieceType::Soldier,
            Color::Black,
        );

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

    #[test]
    fn test_mate_score_decay() {
        // Test that the mate score decay algorithm works correctly
        // Shallower mates (lower ply) should have HIGHER score
        // Deeper mates (higher ply) should have LOWER penalty (less negative score)

        let config = Arc::new(EngineConfig::default());
        let engine = AlphaBetaEngine::new(config.clone());

        // Default decay factor is 0.85
        // mate_score = 30000
        // score = -30000 * 0.85^ply

        let score_ply_1 = engine.calculate_mate_score_for_test(1);
        let score_ply_3 = engine.calculate_mate_score_for_test(3);
        let score_ply_5 = engine.calculate_mate_score_for_test(5);
        let score_ply_7 = engine.calculate_mate_score_for_test(7);

        println!("Mate score at ply 1: {}", score_ply_1);
        println!("Mate score at ply 3: {}", score_ply_3);
        println!("Mate score at ply 5: {}", score_ply_5);
        println!("Mate score at ply 7: {}", score_ply_7);

        // All scores should be positive (mate is good for us)
        assert!(score_ply_1 > 0, "Score at ply 1 should be positive");
        assert!(score_ply_3 > 0, "Score at ply 3 should be positive");
        assert!(score_ply_5 > 0, "Score at ply 5 should be positive");
        assert!(score_ply_7 > 0, "Score at ply 7 should be positive");

        // Faster mates should have higher scores
        // Score = Base - Ply
        // Ply 1: 300000 - 1 = 299999
        // Ply 3: 300000 - 3 = 299997
        // So Ply 1 > Ply 3
        assert!(
            score_ply_1 > score_ply_3,
            "Ply 1 ({}) should be better than ply 3 ({})",
            score_ply_1,
            score_ply_3
        );
        assert!(
            score_ply_3 > score_ply_5,
            "Ply 3 ({}) should be better than ply 5 ({})",
            score_ply_3,
            score_ply_5
        );
        assert!(
            score_ply_5 > score_ply_7,
            "Ply 5 ({}) should be better than ply 7 ({})",
            score_ply_5,
            score_ply_7
        );

        // Check differences
        let diff_1_3 = score_ply_1 - score_ply_3;
        let diff_3_5 = score_ply_3 - score_ply_5;
        let diff_5_7 = score_ply_5 - score_ply_7;

        println!("Difference ply 1->3: {}", diff_1_3);
        println!("Difference ply 3->5: {}", diff_3_5);
        println!("Difference ply 5->7: {}", diff_5_7);

        // Differences should be positive (score decreases with depth)
        assert!(diff_1_3 > 0);
        assert!(diff_3_5 > 0);
        assert!(diff_5_7 > 0);
    }
}
