use crate::logic::game::GameState;
use crate::logic::rules::MoveError;

#[test]
fn test_three_fold_repetition() {
    let mut game = GameState::new();

    // 0. Start: Hash A (Count 1)

    // 1. Red (0,0) -> (1,0)
    // Hash B
    assert!(game.make_move(0, 0, 1, 0).is_ok());

    // 2. Black (9,0) -> (8,0)
    // Hash C
    assert!(game.make_move(9, 0, 8, 0).is_ok());

    // 3. Red (1,0) -> (0,0)
    // Hash D (similar to C but Red back)
    assert!(game.make_move(1, 0, 0, 0).is_ok());

    // 4. Black (8,0) -> (9,0)
    // Hash E == A (Count 2)
    assert!(game.make_move(8, 0, 9, 0).is_ok());

    // 5. Red (0,0) -> (1,0)
    // Hash F == B (Count 2)
    assert!(game.make_move(0, 0, 1, 0).is_ok());

    // 6. Black (9,0) -> (8,0)
    // Hash G == C (Count 2)
    assert!(game.make_move(9, 0, 8, 0).is_ok());

    // 7. Red (1,0) -> (0,0)
    // Hash H == D (Count 2)
    assert!(game.make_move(1, 0, 0, 0).is_ok());

    // 8. Black (8,0) -> (9,0)
    // Hash I == A (Count 3) -> Should Fail
    let result = game.make_move(8, 0, 9, 0);
    assert_eq!(result, Err(MoveError::ThreeFoldRepetition));
}
