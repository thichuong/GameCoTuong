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

#[test]
fn test_engine_filters_repetition_move_at_root() {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use std::sync::Arc;

    let mut game = GameState::new();
    let m = |r, c| BoardCoordinate::new(r, c).unwrap();

    // 0. Start: Hash A (Count 1)
    // 1. Red (0,0) -> (1,0)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    // 2. Black (9,0) -> (8,0)
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    // 3. Red (1,0) -> (0,0)
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    // 4. Black (8,0) -> (9,0) - Hash A (Count 2)
    assert!(game.make_move(m(8, 0), m(9, 0)).is_ok());
    // 5. Red (0,0) -> (1,0)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    // 6. Black (9,0) -> (8,0)
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    // 7. Red (1,0) -> (0,0)
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());

    // Now it's Black's turn. Moving (8,0) -> (9,0) would repeat Hash A for the 3rd time.
    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);

    let (best_move, _) = engine.search(&game, SearchLimit::Depth(2), &[]).unwrap();

    // Verify engine did NOT choose the illegal repetition move (8,0) -> (9,0)
    let is_repetition_move = best_move.from_row == 8
        && best_move.from_col == 0
        && best_move.to_row == 9
        && best_move.to_col == 0;

    assert!(
        !is_repetition_move,
        "Engine must filter out 3-fold repetition move at root"
    );

    // Verify making the best move chosen by AI succeeds without repetition error
    let from =
        BoardCoordinate::new(best_move.from_row as usize, best_move.from_col as usize).unwrap();
    let to = BoardCoordinate::new(best_move.to_row as usize, best_move.to_col as usize).unwrap();
    assert!(game.make_move(from, to).is_ok());

    // Verify GameState JSON serialization preserves move history and hashes
    let json = serde_json::to_string(&game).unwrap();
    let restored: GameState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.history.len(), game.history.len());
    assert_eq!(restored.board.zobrist_hash, game.board.zobrist_hash);
    for (orig, rest) in game.history.iter().zip(restored.history.iter()) {
        assert_eq!(orig.hash, rest.hash);
        assert_eq!(orig.from, rest.from);
        assert_eq!(orig.to, rest.to);
    }
}
