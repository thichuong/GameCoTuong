#[cfg(test)]
mod tests {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use crate::logic::board::Board;
    use crate::logic::game::GameState;
    use std::sync::Arc;

    use crate::logic::board::{Color, Piece, PieceType};

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
        // Red: King(0,4), Rook(0,0), Horse(2,2), Pawn(6,4)
        // Red: King(0,4), Rook(0,0), Horse(2,2), Pawn(6,4)
        // Black: King(9,4), Advisor(9,3), Advisor(9,5), Cannon(7,4), Rook(9,8)
        let mut board = Board {
            bitboards: [0; 14],
            occupied: 0,
            grid: [None; 90],
            occupied_rows: [0; 10],
            occupied_cols: [0; 9],
            zobrist_hash: 0,
            red_material: 0,
            black_material: 0,
            red_pst: 0,
            black_pst: 0,
        };

        // Red Pieces
        board.set_piece(
            0,
            4,
            Some(Piece {
                piece_type: PieceType::General,
                color: Color::Red,
            }),
        );
        board.set_piece(
            0,
            0,
            Some(Piece {
                piece_type: PieceType::Chariot,
                color: Color::Red,
            }),
        );
        board.set_piece(
            2,
            2,
            Some(Piece {
                piece_type: PieceType::Horse,
                color: Color::Red,
            }),
        );
        board.set_piece(
            6,
            4,
            Some(Piece {
                piece_type: PieceType::Soldier,
                color: Color::Red,
            }),
        );

        // Black Pieces
        board.set_piece(
            9,
            4,
            Some(Piece {
                piece_type: PieceType::General,
                color: Color::Black,
            }),
        );
        board.set_piece(
            9,
            3,
            Some(Piece {
                piece_type: PieceType::Advisor,
                color: Color::Black,
            }),
        );
        board.set_piece(
            9,
            5,
            Some(Piece {
                piece_type: PieceType::Advisor,
                color: Color::Black,
            }),
        );
        board.set_piece(
            7,
            4,
            Some(Piece {
                piece_type: PieceType::Cannon,
                color: Color::Black,
            }),
        );
        board.set_piece(
            9,
            8,
            Some(Piece {
                piece_type: PieceType::Chariot,
                color: Color::Black,
            }),
        );

        board.zobrist_hash = board.calculate_initial_hash();
        board.calculate_initial_score();

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
    fn bench_dynamic_limiting() {
        println!("--- Benchmarking Dynamic Limiting (Method 0) ---");
        let mut config = EngineConfig::default();
        config.pruning_method = 0; // Dynamic Limiting
        let config = Arc::new(config);
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
            println!("Dynamic Limiting Depth 6 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }

    #[test]
    fn bench_aggressive() {
        println!("--- Benchmarking Aggressive Pruning (Method 2) ---");
        let mut config = EngineConfig::default();
        config.pruning_method = 2; // Both (Aggressive)
        let config = Arc::new(config);
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
            println!("Aggressive Pruning Depth 6 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }

    #[test]
    fn bench_lmr() {
        println!("--- Benchmarking LMR (Method 1) ---");
        let mut config = EngineConfig::default();
        config.pruning_method = 1; // LMR
        let config = Arc::new(config);
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
            println!("LMR Depth 6 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;
            println!("NPS: {nps}");
        } else {
            panic!("Search returned None");
        }
    }
}
