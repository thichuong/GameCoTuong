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
    fn bench_standard_nnue_search_performance() {
        println!("=== Standard NNUE Search Performance Benchmark ===");
        let board = Board::new();
        let game_state = GameState {
            board,
            turn: Color::Red,
            ..Default::default()
        };

        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config);
        engine.search(&game_state, SearchLimit::Depth(2), &[]);
        let start = std::time::Instant::now();
        let res = engine.search(&game_state, SearchLimit::Depth(6), &[]);
        let dur = start.elapsed();
        let (_, stats) = res.expect("Standard NNUE search succeeded");
        let nps = (stats.nodes as f64 / dur.as_secs_f64()) as u64;
        println!(
            "Standard NNUE (Depth 6): nodes={}, time={:?}, NPS={}",
            stats.nodes, dur, nps
        );
        assert!(stats.nodes > 0);
    }

    #[test]
    fn test_subtree_reuse_efficiency() {
        println!("--- Benchmarking Subtree & TT Reuse Efficiency ---");
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config.clone());
        let mut game_state = GameState::new();

        // 1. First move search from opening position at Depth 5
        let (mv1, _stats1) = engine
            .search(&game_state, SearchLimit::Depth(5), &[])
            .expect("First move search");

        let from1 = BoardCoordinate::new(mv1.from_row as usize, mv1.from_col as usize).unwrap();
        let to1 = BoardCoordinate::new(mv1.to_row as usize, mv1.to_col as usize).unwrap();
        game_state.make_move(from1, to1).expect("Make first move");

        // 2. Opponent makes a move (e.g. Knight to central file or first legal move)
        let opp_moves = game_state.get_strictly_legal_moves();
        assert!(!opp_moves.is_empty(), "Opponent must have legal moves");
        let opp_mv = opp_moves[0];
        let from2 =
            BoardCoordinate::new(opp_mv.from_row as usize, opp_mv.from_col as usize).unwrap();
        let to2 = BoardCoordinate::new(opp_mv.to_row as usize, opp_mv.to_col as usize).unwrap();
        game_state
            .make_move(from2, to2)
            .expect("Make opponent move");

        // 3. Search next turn WITH reused subtree in TT
        let (_mv_reuse, stats_reuse) = engine
            .search(&game_state, SearchLimit::Depth(5), &[])
            .expect("Search with reused subtree");

        // 4. Search next turn WITHOUT reused subtree (cold start after clear_tt)
        let mut engine_fresh = AlphaBetaEngine::new(config);
        engine_fresh.clear_tt();
        let (_mv_fresh, stats_fresh) = engine_fresh
            .search(&game_state, SearchLimit::Depth(5), &[])
            .expect("Search with fresh TT");

        println!(
            "Subtree Reuse: nodes searched = {} vs Fresh Search: nodes searched = {}",
            stats_reuse.nodes, stats_fresh.nodes
        );

        let node_saving_pct = (1.0 - (stats_reuse.nodes as f64 / stats_fresh.nodes as f64)) * 100.0;
        println!(
            "Node search reduction from subtree reuse: {:.2}%",
            node_saving_pct
        );

        assert!(
            stats_reuse.nodes <= stats_fresh.nodes,
            "Reused subtree must search fewer or equal nodes than starting from scratch! (reused: {}, fresh: {})",
            stats_reuse.nodes,
            stats_fresh.nodes
        );
    }
}
