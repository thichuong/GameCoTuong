use super::*;
use cotuong_core::logic::board::{Board, Color};
use shared::ServerMessage;
use std::time::Duration;
use tokio::sync::mpsc;

// Helper to receive next message with timeout
async fn expect_msg_timeout(rx: &mut mpsc::UnboundedReceiver<ServerMessage>) -> ServerMessage {
    tokio::time::timeout(Duration::from_millis(1500), rx.recv())
        .await
        .expect("Timed out waiting for message")
        .expect("Channel closed")
}

// Drain setup messages (MatchFound, GameStart, Waiting)
async fn drain_setup_messages(rx: &mut mpsc::UnboundedReceiver<ServerMessage>) {
    loop {
        match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Some(msg)) => match msg {
                ServerMessage::GameStart(_) => break,
                _ => continue,
            },
            _ => break,
        }
    }
}

#[tokio::test]
async fn test_happy_path_distributed_validation() {
    let app_state = AppState::new();
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    let p1_id = "p1".to_string();
    let p2_id = "p2".to_string();

    app_state.add_player(p1_id.clone(), tx1);
    app_state.add_player(p2_id.clone(), tx2);

    // Matchmake
    app_state.find_match(p1_id.clone()).await;
    app_state.find_match(p2_id.clone()).await;

    // Drain setup
    drain_setup_messages(&mut rx1).await;
    drain_setup_messages(&mut rx2).await;

    let game_id = app_state
        .player_to_game
        .get(&p1_id)
        .expect("Game should exist")
        .value()
        .clone();

    let game_lock = app_state.games.get(&game_id).expect("Game session missing");
    let game = game_lock.read().await;
    let red_id = game.red_player.clone();
    let is_p1_red = red_id == p1_id;
    drop(game); // Release lock

    // Identify Black
    let black_id = if is_p1_red {
        p2_id.clone()
    } else {
        p1_id.clone()
    };

    // Generate valid move (Red)
    let board = Board::new();
    let gen = cotuong_core::logic::generator::MoveGenerator::new();
    let moves = gen.generate_moves(&board, Color::Red);
    let valid_move = moves.first().expect("Should have moves").clone();

    // Calculate expected FEN
    let mut test_board = board.clone();
    test_board.apply_move(&valid_move, Color::Red);
    let expected_fen = test_board.to_fen_string(Color::Black);

    // P1 sends MakeMove
    app_state
        .handle_move(red_id.clone(), valid_move.clone(), expected_fen.clone())
        .await;

    // Verify Pending
    {
        let game_lock = app_state.games.get(&game_id).unwrap();
        let game = game_lock.read().await;
        assert!(game.pending_move.is_some());
        if let Some((pid, m, f)) = &game.pending_move {
            assert_eq!(pid, &red_id);
            assert_eq!(m.from_row, valid_move.from_row);
            assert_eq!(f, &expected_fen);
        }
    }

    // Opponent (Black) should receive OpponentMove
    let mut opponent_rx = if is_p1_red { &mut rx2 } else { &mut rx1 };

    match expect_msg_timeout(&mut opponent_rx).await {
        ServerMessage::OpponentMove { move_data, fen } => {
            assert_eq!(move_data.from_row, valid_move.from_row);
            assert_eq!(fen, expected_fen);
        }
        other => match expect_msg_timeout(&mut opponent_rx).await {
            ServerMessage::OpponentMove { move_data, fen } => {
                assert_eq!(move_data.from_row, valid_move.from_row);
                assert_eq!(fen, expected_fen);
            }
            _ => panic!("Expected OpponentMove, got {:?}", other),
        },
    }

    // Opponent Verifies (TRUE)
    app_state
        .handle_verify_move(black_id.clone(), expected_fen.clone(), true)
        .await;

    // Verify Server State Updated
    {
        let game_lock = app_state.games.get(&game_id).unwrap();
        let game = game_lock.read().await;
        assert!(game.pending_move.is_none());
        assert_eq!(game.turn, Color::Black);
        assert_eq!(game.board.to_fen_string(Color::Black), expected_fen);
    }
}

#[tokio::test]
async fn test_conflict_resolution() {
    let app_state = AppState::new();
    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    let p1_id = "p1".to_string();
    let p2_id = "p2".to_string();

    app_state.add_player(p1_id.clone(), tx1);
    app_state.add_player(p2_id.clone(), tx2);

    app_state.find_match(p1_id.clone()).await;
    app_state.find_match(p2_id.clone()).await;

    drain_setup_messages(&mut rx1).await;
    drain_setup_messages(&mut rx2).await;

    let game_id = app_state
        .player_to_game
        .get(&p1_id)
        .unwrap()
        .value()
        .clone();
    let game_lock = app_state.games.get(&game_id).unwrap();
    let game = game_lock.read().await;
    let red_id = game.red_player.clone();
    let is_p1_red = red_id == p1_id;
    let black_id = if is_p1_red {
        p2_id.clone()
    } else {
        p1_id.clone()
    };
    drop(game);

    // Valid Move
    let board = Board::new();
    let gen = cotuong_core::logic::generator::MoveGenerator::new();
    let moves = gen.generate_moves(&board, Color::Red);
    let valid_move = moves.first().unwrap().clone();

    // Correct FEN
    let mut test_board = board.clone();
    test_board.apply_move(&valid_move, Color::Red);
    let valid_fen = test_board.to_fen_string(Color::Black);

    // Incorrect FEN
    let initial_fen = board.to_fen_string(Color::Red);

    // P1 sends VALID move but claims INITIAL FEN
    app_state
        .handle_move(red_id.clone(), valid_move.clone(), initial_fen.clone())
        .await;

    let (p1_rx, p2_rx) = if is_p1_red {
        (&mut rx1, &mut rx2)
    } else {
        (&mut rx2, &mut rx1)
    };

    loop {
        let msg = expect_msg_timeout(&mut *p2_rx).await;
        match msg {
            ServerMessage::OpponentMove { fen, .. } => {
                assert_eq!(fen, initial_fen);
                break;
            }
            _ => continue,
        }
    }

    // P2 reports CONFLICT (false)
    app_state
        .handle_verify_move(black_id.clone(), valid_fen.clone(), false)
        .await;

    // Check P1 (Red) receives correction
    loop {
        let msg = expect_msg_timeout(&mut *p1_rx).await;
        match msg {
            ServerMessage::GameStateCorrection { fen, turn } => {
                assert_eq!(fen, valid_fen);
                assert_eq!(turn, Color::Black);
                break;
            }
            _ => continue,
        }
    }

    // Check P2 (Black) receives correction
    loop {
        let msg = expect_msg_timeout(&mut *p2_rx).await;
        match msg {
            ServerMessage::GameStateCorrection { fen, turn } => {
                assert_eq!(fen, valid_fen);
                assert_eq!(turn, Color::Black);
                break;
            }
            _ => continue,
        }
    }

    // Verify Server State Updated
    {
        let game_lock = app_state.games.get(&game_id).unwrap();
        let game = game_lock.read().await;
        assert!(game.pending_move.is_none());
        assert_eq!(game.board.to_fen_string(Color::Black), valid_fen);
    }
}
