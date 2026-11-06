#[cfg(test)]
mod tests {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use crate::logic::board::{Board, BoardCoordinate, Color, Piece, PieceType};
    use crate::logic::game::GameState;
    use std::sync::Arc;

    #[test]
    fn bench_opening() {
        println!("--- Benchmarking Opening ---");
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config);
        let board = Board::new();
        let game_state = GameState {
            board,
            turn: Color::Red,
            ..Default::default()
        };

        // Warmup
        engine.search(&game_state, SearchLimit::Depth(2), &[]);

        let start = std::time::Instant::now();
        let result = engine.search(&game_state, SearchLimit::Depth(5), &[]); // Depth 5 for opening
        let duration = start.elapsed();

        if let Some((_mv, stats)) = result {
            println!("Opening Depth 5 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }

    #[test]
    fn bench_endgame() {
        println!("--- Benchmarking Endgame ---");
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config);

        // Setup Endgame Position
        let mut board = Board {
            bitboards: [0; 14],
            occupied: 0,
            grid: [None; 90],
            occupied_rows: [0; 10],
            occupied_cols: [0; 9],
            zobrist_hash: 0,
            nnue_acc: Default::default(),
        };

        // Helper
        let mut set = |r, c, pt, color| {
            board.set_piece(
                BoardCoordinate::new(r, c).unwrap(),
                Some(Piece {
                    piece_type: pt,
                    color,
                }),
            );
        };

        // Red Pieces
        set(0, 4, PieceType::General, Color::Red);
        set(0, 0, PieceType::Chariot, Color::Red);
        set(2, 2, PieceType::Horse, Color::Red);
        set(6, 4, PieceType::Soldier, Color::Red);

        // Black Pieces
        set(9, 4, PieceType::General, Color::Black);
        set(9, 3, PieceType::Advisor, Color::Black);
        set(9, 5, PieceType::Advisor, Color::Black);
        set(7, 4, PieceType::Cannon, Color::Black);
        set(9, 8, PieceType::Chariot, Color::Black);

        board.zobrist_hash = board.calculate_initial_hash();

        let game_state = GameState {
            board,
            turn: Color::Red,
            ..Default::default()
        };

        // Warmup
        engine.search(&game_state, SearchLimit::Depth(2), &[]);

        let start = std::time::Instant::now();
        let result = engine.search(&game_state, SearchLimit::Depth(7), &[]); // Deeper search for endgame
        let duration = start.elapsed();

        if let Some((_mv, stats)) = result {
            println!("Endgame Depth 7 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }

    #[test]
    fn bench_search_depth_6() {
        println!("--- Benchmarking Search Depth 6 ---");
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config);

        let board = Board::new();
        let game_state = GameState {
            board,
            turn: Color::Red,
            ..Default::default()
        };

        // Warmup
        engine.search(&game_state, SearchLimit::Depth(2), &[]);

        let start = std::time::Instant::now();
        let result = engine.search(&game_state, SearchLimit::Depth(6), &[]);
        let duration = start.elapsed();

        if let Some((_mv, stats)) = result {
            println!("Search Depth 6 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }
}
