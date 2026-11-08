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

    #[test]
    fn bench_model_int16_vs_int8_comparison() {
        println!("=== Comparing Model Int16 vs Int8 Search Performance ===");
        let board = Board::new();
        let game_state = GameState {
            board,
            turn: Color::Red,
            ..Default::default()
        };

        // 1. Benchmark Int16 (default active dims 192)
        let config_int16 = Arc::new(EngineConfig {
            model_type: "int16".to_string(),
            ..Default::default()
        });
        let mut engine_int16 = AlphaBetaEngine::new(config_int16);
        engine_int16.search(&game_state, SearchLimit::Depth(2), &[]);
        let start_i16 = std::time::Instant::now();
        let res_i16 = engine_int16.search(&game_state, SearchLimit::Depth(6), &[]);
        let dur_i16 = start_i16.elapsed();
        let (_, stats_i16) = res_i16.expect("Int16 search succeeded");
        let nps_i16 = (stats_i16.nodes as f64 / dur_i16.as_secs_f64()) as u64;
        println!(
            "Model INT16 (Depth 6): nodes={}, time={:?}, NPS={}",
            stats_i16.nodes, dur_i16, nps_i16
        );

        // 2. Benchmark Int8 (active dims 64)
        let config_int8 = Arc::new(EngineConfig {
            model_type: "int8".to_string(),
            ..Default::default()
        });
        let mut engine_int8 = AlphaBetaEngine::new(config_int8);
        engine_int8.search(&game_state, SearchLimit::Depth(2), &[]);
        let start_i8 = std::time::Instant::now();
        let res_i8 = engine_int8.search(&game_state, SearchLimit::Depth(6), &[]);
        let dur_i8 = start_i8.elapsed();
        let (_, stats_i8) = res_i8.expect("Int8 search succeeded");
        let nps_i8 = (stats_i8.nodes as f64 / dur_i8.as_secs_f64()) as u64;
        println!(
            "Model INT8  (Depth 6): nodes={}, time={:?}, NPS={}",
            stats_i8.nodes, dur_i8, nps_i8
        );
        println!(
            "Speedup ratio (Int8 NPS / Int16 NPS): {:.2}x",
            nps_i8 as f64 / nps_i16 as f64
        );
    }
}
