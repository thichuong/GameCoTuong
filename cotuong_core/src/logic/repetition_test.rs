use crate::logic::board::BoardCoordinate;
use crate::logic::game::GameState;
use crate::logic::rules::MoveError;

#[test]
fn test_three_fold_repetition() {
    let mut game = GameState::new();

    let m = |r, c| BoardCoordinate::new(r, c).unwrap();

    // 0. Start: Hash A (Count 1)

    // 1. Red (0,0) -> (1,0)
    // Hash B
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());

    // 2. Black (9,0) -> (8,0)
    // Hash C
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());

    // 3. Red (1,0) -> (0,0)
    // Hash D (similar to C but Red back)
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());

    // 4. Black (8,0) -> (9,0)
    // Hash E == A (Count 2)
    assert!(game.make_move(m(8, 0), m(9, 0)).is_ok());

    // 5. Red (0,0) -> (1,0)
    // Hash F == B (Count 2)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());

    // 6. Black (9,0) -> (8,0)
    // Hash G == C (Count 2)
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());

    // 7. Red (1,0) -> (0,0)
    // Hash H == D (Count 2)
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());

    // 8. Black (8,0) -> (9,0)
    // Hash I == A (Count 3) -> Should Fail
    let result = game.make_move(m(8, 0), m(9, 0));
    assert_eq!(result, Err(MoveError::ThreeFoldRepetition));
}

#[test]
fn test_engine_excludes_moves() {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use std::sync::Arc;

    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);
    let game = GameState::new();

    // 1. Search for best move (usually Red Pawn 2->5 or Cannon 2->5)
    // Let's just see what it returns first
    let (best_move, _) = engine.search(&game, SearchLimit::Depth(2), &[]).unwrap();

    // 2. Exclude that move and search again
    let excluded = vec![best_move];
    let (next_best, _) = engine
        .search(&game, SearchLimit::Depth(2), &excluded)
        .unwrap();

    // 3. Assert they are different
    assert_ne!(
        best_move, next_best,
        "Engine should pick a different move when best move is excluded"
    );

    // 4. Assert next_best is NOT the excluded move
    assert!(
        next_best.from_row != best_move.from_row
            || next_best.from_col != best_move.from_col
            || next_best.to_row != best_move.to_row
            || next_best.to_col != best_move.to_col
    );
}
