use crate::logic::board::BoardCoordinate;
use crate::logic::game::GameState;
use crate::logic::rules::MoveError;

#[test]
fn test_three_fold_repetition() {
    let mut game = GameState::new();

    let m = |r, c| BoardCoordinate::new(r, c).unwrap();

    // Lặp lại chu kỳ tối đa 3 lần được phép, lần thứ 4 bị cấm:
    // Chu kỳ 1 (Nước 1-4)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    assert!(game.make_move(m(8, 0), m(9, 0)).is_ok());

    // Chu kỳ 2 (Nước 5-8): Lặp lại lần 2 (được phép)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    assert!(game.make_move(m(8, 0), m(9, 0)).is_ok());

    // Chu kỳ 3 (Nước 9-12): Lặp lại lần 3 (được phép)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    assert!(game.make_move(m(8, 0), m(9, 0)).is_ok());

    // Nước 13: Đỏ toan đi (0,0) -> (1,0) lần thứ 4 (vượt quá tối đa 3 lần) -> BỊ CẤM!
    assert!(
        !game.is_move_legal(m(0, 0), m(1, 0)),
        "Repetition move exceeding 3 times must be rejected by is_move_legal"
    );
    let strictly_legal = game.get_strictly_legal_moves();
    assert!(
        !strictly_legal
            .iter()
            .any(|mv| mv.from_row == 0 && mv.from_col == 0 && mv.to_row == 1 && mv.to_col == 0),
        "Repetition move exceeding 3 times must be excluded from get_strictly_legal_moves"
    );

    let result = game.make_move(m(0, 0), m(1, 0));
    assert_eq!(result, Err(MoveError::ThreeFoldRepetition));
}

#[test]
fn test_single_piece_back_and_forth_forbidden() {
    let mut game = GameState::new();
    let m = |r, c| BoardCoordinate::new(r, c).unwrap();

    // Lần 1:
    assert!(game.make_move(m(2, 1), m(2, 4)).is_ok());
    assert!(game.make_move(m(9, 8), m(8, 8)).is_ok());
    assert!(game.make_move(m(2, 4), m(2, 1)).is_ok());
    assert!(game.make_move(m(8, 8), m(9, 8)).is_ok());

    // Lần 2 (lặp lại lần 2 - được phép):
    assert!(game.make_move(m(2, 1), m(2, 4)).is_ok());
    assert!(game.make_move(m(9, 8), m(8, 8)).is_ok());
    assert!(game.make_move(m(2, 4), m(2, 1)).is_ok());
    assert!(game.make_move(m(8, 8), m(9, 8)).is_ok());

    // Lần 3 (lặp lại lần 3 - được phép):
    assert!(game.make_move(m(2, 1), m(2, 4)).is_ok());
    assert!(game.make_move(m(9, 8), m(8, 8)).is_ok());
    assert!(game.make_move(m(2, 4), m(2, 1)).is_ok());
    assert!(game.make_move(m(8, 8), m(9, 8)).is_ok());

    // Lần 4: Đỏ toan đi (2,1) -> (2,4) lần thứ 4 (vượt quá tối đa 3 lần) -> BỊ CẤM!
    assert!(
        !game.is_move_legal(m(2, 1), m(2, 4)),
        "Single piece back-and-forth move exceeding 3 times must be rejected"
    );

    let strictly_legal = game.get_strictly_legal_moves();
    assert!(
        !strictly_legal
            .iter()
            .any(|mv| mv.from_row == 2 && mv.from_col == 1 && mv.to_row == 2 && mv.to_col == 4),
        "Single piece back-and-forth move exceeding 3 times must be excluded"
    );

    let result = game.make_move(m(2, 1), m(2, 4));
    assert_eq!(
        result,
        Err(MoveError::ThreeFoldRepetition),
        "make_move must reject 4th repeat"
    );
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

    // Chu kỳ 1 (1-4)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    let res = game.make_move(m(8, 0), m(9, 0));
    if res.is_err() {
        eprintln!("Move 8 Error: {:?}", res);
    }
    assert!(res.is_ok());

    // Chu kỳ 2 (5-8)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    let res = game.make_move(m(8, 0), m(9, 0));
    if res.is_err() {
        eprintln!("Move 8 Error: {:?}", res);
    }
    assert!(res.is_ok());

    // Chu kỳ 3 (9-12)
    assert!(game.make_move(m(0, 0), m(1, 0)).is_ok());
    assert!(game.make_move(m(9, 0), m(8, 0)).is_ok());
    assert!(game.make_move(m(1, 0), m(0, 0)).is_ok());
    let res = game.make_move(m(8, 0), m(9, 0));
    if res.is_err() {
        eprintln!("Move 8 Error: {:?}", res);
    }
    assert!(res.is_ok());

    // Bây giờ đến lượt Đỏ. Nước đi (0,0) -> (1,0) đã được lặp lại 3 lần, lần thứ 4 là cấm.
    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);

    let (best_move, _) = engine.search(&game, SearchLimit::Depth(2), &[]).unwrap();

    // Xác nhận engine KHÔNG chọn nước cờ lặp lại vi phạm (0,0) -> (1,0)
    let is_repetition_move = best_move.from_row == 0
        && best_move.from_col == 0
        && best_move.to_row == 1
        && best_move.to_col == 0;

    assert!(
        !is_repetition_move,
        "Engine must filter out repetition move at root"
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

#[test]
fn test_perpetual_check_forbidden_for_player_and_engine() {
    use crate::engine::config::EngineConfig;
    use crate::engine::search::AlphaBetaEngine;
    use crate::engine::{SearchLimit, Searcher};
    use crate::logic::board::{Board, Color, PieceType};
    use std::sync::Arc;

    let mut board = Board::new();
    board.clear();
    let m = |r, c| BoardCoordinate::new(r, c).unwrap();

    // Red: General at (0, 3), Chariot at (9, 1), Extra Rook at (0, 0)
    board.add_piece(m(0, 3), PieceType::General, Color::Red);
    board.add_piece(m(9, 1), PieceType::Chariot, Color::Red);
    board.add_piece(m(0, 0), PieceType::Chariot, Color::Red);

    // Black: General at (9, 4), Advisors at (9, 5) and (8, 5)
    board.add_piece(m(9, 4), PieceType::General, Color::Black);
    board.add_piece(m(9, 5), PieceType::Advisor, Color::Black);
    board.add_piece(m(8, 5), PieceType::Advisor, Color::Black);

    board.zobrist_hash = board.calculate_initial_hash();

    let initial_hash = board.zobrist_hash;
    let mut game = GameState {
        board,
        turn: Color::Red,
        initial_hash,
        ..Default::default()
    };

    // Chu kỳ 1 (1-4)
    assert!(game.make_move(m(9, 1), m(9, 2)).is_ok());
    assert!(game.make_move(m(9, 4), m(8, 4)).is_ok());
    assert!(game.make_move(m(9, 2), m(8, 2)).is_ok());
    assert!(game.make_move(m(8, 4), m(9, 4)).is_ok());

    // Chu kỳ 2 (5-8)
    assert!(game.make_move(m(8, 2), m(9, 2)).is_ok());
    assert!(game.make_move(m(9, 4), m(8, 4)).is_ok());
    assert!(game.make_move(m(9, 2), m(8, 2)).is_ok());
    assert!(game.make_move(m(8, 4), m(9, 4)).is_ok());

    // Chu kỳ 3 (9-12)
    assert!(game.make_move(m(8, 2), m(9, 2)).is_ok());
    assert!(game.make_move(m(9, 4), m(8, 4)).is_ok());
    assert!(game.make_move(m(9, 2), m(8, 2)).is_ok());
    assert!(game.make_move(m(8, 4), m(9, 4)).is_ok());

    // Nước 13: Đỏ toan đi (8, 2) -> (9, 2) chiếu lần thứ 4 (vượt quá 3 lần) -> BỊ CẤM!
    assert!(
        !game.is_move_legal(m(8, 2), m(9, 2)),
        "Perpetual check move exceeding 3 times must be rejected by is_move_legal"
    );

    let strictly_legal = game.get_strictly_legal_moves();
    assert!(
        !strictly_legal
            .iter()
            .any(|mv| mv.from_row == 8 && mv.from_col == 2 && mv.to_row == 9 && mv.to_col == 2),
        "Perpetual check move exceeding 3 times must be excluded from get_strictly_legal_moves"
    );

    let move_result = game.make_move(m(8, 2), m(9, 2));
    assert!(
        move_result.is_err(),
        "make_move must reject perpetual check move"
    );

    // Engine test: AI search must filter perpetual check at root and never choose it
    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);
    let (best_move, _) = engine.search(&game, SearchLimit::Depth(3), &[]).unwrap();

    let is_perp_check_chosen = best_move.from_row == 8
        && best_move.from_col == 2
        && best_move.to_row == 9
        && best_move.to_col == 2;
    assert!(
        !is_perp_check_chosen,
        "Engine search must not choose perpetual check move (filtered at root)"
    );
}
