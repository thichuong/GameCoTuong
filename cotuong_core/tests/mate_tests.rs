#[cfg(test)]
mod tests {
    use cotuong_core::engine::config::EngineConfig;
    use cotuong_core::engine::search::AlphaBetaEngine;
    use cotuong_core::engine::{SearchLimit, Searcher};
    use cotuong_core::logic::board::{BoardCoordinate, Color, PieceType};
    use std::sync::Arc;

    use cotuong_core::logic::game::GameState;

    fn setup_mate_in_1_pos() -> GameState {
        let mut game = GameState::new();
        game.board.clear();

        // Red King at (0, 4)
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(0, 4) },
            PieceType::General,
            Color::Red,
        );
        // Red Cannon at (2, 4)
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(2, 4) },
            PieceType::Cannon,
            Color::Red,
        );
        // Red Pawn at (4, 4)
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(4, 4) },
            PieceType::Soldier,
            Color::Red,
        );

        // Black King at (9, 4)
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(9, 4) },
            PieceType::General,
            Color::Black,
        );
        // Black Advisors
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(9, 3) },
            PieceType::Advisor,
            Color::Black,
        );
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(9, 5) },
            PieceType::Advisor,
            Color::Black,
        );
        // Black Pawn (8, 4) blocks escape
        game.board.add_piece(
            unsafe { BoardCoordinate::new_unchecked(8, 4) },
            PieceType::Soldier,
            Color::Black,
        );

        // Ensure hashes are updated
        game.board.zobrist_hash = game.board.calculate_initial_hash();
        game.board.calculate_initial_score();

        game
    }

    #[test]
    fn test_mate_in_1_search() {
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config.clone());
        let mut game = setup_mate_in_1_pos();

        // Red to move
        // Expect move: Pawn (4, 4) -> (5, 4) (Creates Cannon mount for Checkmate)

        // Search with limit depth 3
        let limit = SearchLimit::Depth(3);
        let result = engine.search(&mut game, limit, &[]);

        if let Some((mv, _stats)) = result {
            // Verify move from (4,4) to (5,4)
            assert_eq!(mv.from_row, 4);
            assert_eq!(mv.from_col, 4);
            assert_eq!(mv.to_row, 5);
            assert_eq!(mv.to_col, 4);

            // We can't check score directly from SearchStats, but finding the move is good.
        } else {
            panic!("Engine returned no move for mate-in-1 position");
        }
    }
}
