#[cfg(test)]
mod tests {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use crate::logic::board::Board;
    use crate::logic::game::GameState;
    use std::sync::Arc;

    #[test]
    fn test_engine_performance() {
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config);
        let board = Board::new();
        let game_state = GameState {
            board,
            turn: crate::logic::board::Color::Red,
            // Add other fields if needed, or use default if GameState has it
            ..Default::default()
        };

        // Warmup
        engine.search(&game_state, SearchLimit::Depth(2));

        // Measure Depth 4
        let start = std::time::Instant::now();
        let result = engine.search(&game_state, SearchLimit::Depth(4));
        let duration = start.elapsed();

        if let Some((_mv, stats)) = result {
            println!("Depth 4 stats: {stats:?}");
            println!("Time taken: {duration:?}");
            // Assert reasonable nodes/time
            // With TT and Move Ordering, Depth 4 should be very fast (< 100ms usually for initial position)
            // Previous engine might have been slower.
        } else {
            panic!("Search returned None");
        }
    }
}
