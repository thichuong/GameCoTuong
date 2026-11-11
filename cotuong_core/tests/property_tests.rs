//! Property-based tests for cotuong_core.
//!
//! Verifies mathematical and game-rule invariants across random board states,
//! move trajectories, and undo/redo stacks:
//! 1. `apply_move` / `undo_move` exact bit-for-bit invariance
//! 2. Incremental HalfKP NNUE accumulator vs from-scratch recomputation
//! 3. Incremental Zobrist hash vs from-scratch piece hash
//! 4. FEN serialization roundtrip (`to_fen_string` <-> `from_fen`)
//! 5. Legal move equivalence between `MoveGenerator` and `EngineMoveGen`
//! 6. Legal moves strictly guarantee general safety (no self-check, no flying general)
//! 7. UCI coordinate move roundtrip
//! 8. `GameState` undo stack invariance

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

use cotuong_core::engine::movegen::EngineMoveGen;
use cotuong_core::engine::zobrist::ZobristKeys;
use cotuong_core::engine::Move;
use cotuong_core::logic::board::{Board, BoardCoordinate, Color, Piece};
use cotuong_core::logic::game::GameState;
use cotuong_core::logic::generator::MoveGenerator;
use cotuong_core::logic::rules::{could_cause_flying_general, is_flying_general, is_in_check};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn get_seed() -> u64 {
    std::env::var("PROPTEST_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xC0_70_09_20_26)
}

fn get_game_count(default: usize) -> usize {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Plays a single random legal move from the current position.
/// Returns `Some((played_move, captured_piece))` or `None` if the game is over.
fn play_random_legal_move(
    board: &mut Board,
    turn: &mut Color,
    mg: &MoveGenerator,
    rng: &mut StdRng,
) -> Option<(Move, Option<Piece>)> {
    let moves = mg.generate_moves(board, *turn);
    if moves.is_empty() {
        return None;
    }
    let mv = moves[rng.gen_range(0..moves.len())];
    let to_coord = BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize)?;
    let captured = board.get_piece(to_coord);

    board.apply_move(&mv, *turn);
    *turn = turn.opposite();

    Some((mv, captured))
}

#[test]
fn prop_apply_undo_invariance() {
    let seed = get_seed();
    let num_games = get_game_count(150);
    println!("Running prop_apply_undo_invariance with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let mg = MoveGenerator::new();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let mut history: Vec<(Move, Option<Piece>, Color)> = Vec::with_capacity(60);

        let max_plies = rng.gen_range(15..=50);

        for _ in 0..max_plies {
            let moves = mg.generate_moves(&board, turn);
            if moves.is_empty() {
                break;
            }

            let mv = moves[rng.gen_range(0..moves.len())];
            let to_coord =
                BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize).expect("Valid coord");
            let captured = board.get_piece(to_coord);

            // Snapshot state before applying the move
            let before_grid = board.grid;
            let before_bitboards = board.bitboards;
            let before_occupied = board.occupied;
            let before_occupied_rows = board.occupied_rows;
            let before_occupied_cols = board.occupied_cols;
            let before_hash = board.zobrist_hash;
            let before_nnue_red = board.nnue_acc.red;
            let before_nnue_black = board.nnue_acc.black;
            let before_psqt_red = board.nnue_acc.psqt_red;
            let before_psqt_black = board.nnue_acc.psqt_black;

            // Apply move
            board.apply_move(&mv, turn);

            // Immediate single-ply undo
            board.undo_move(&mv, captured, turn);

            // Assert exact bit-for-bit restoration
            assert_eq!(
                board.grid, before_grid,
                "Grid mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.bitboards, before_bitboards,
                "Bitboard mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.occupied, before_occupied,
                "Occupied mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.occupied_rows, before_occupied_rows,
                "Occupied rows mismatch at game {game_idx}"
            );
            assert_eq!(
                board.occupied_cols, before_occupied_cols,
                "Occupied cols mismatch at game {game_idx}"
            );
            assert_eq!(
                board.zobrist_hash, before_hash,
                "Zobrist hash mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.nnue_acc.red, before_nnue_red,
                "NNUE red acc mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.nnue_acc.black, before_nnue_black,
                "NNUE black acc mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.nnue_acc.psqt_red, before_psqt_red,
                "NNUE psqt_red mismatch on undo at game {game_idx}"
            );
            assert_eq!(
                board.nnue_acc.psqt_black, before_psqt_black,
                "NNUE psqt_black mismatch on undo at game {game_idx}"
            );

            // Re-apply move and advance game
            board.apply_move(&mv, turn);
            history.push((mv, captured, turn));
            turn = turn.opposite();
        }

        // Full trajectory unwinding: undo all moves one by one back to starting position
        while let Some((mv, cap, t)) = history.pop() {
            board.undo_move(&mv, cap, t);
        }

        // Board must be back to the fresh initial state
        let fresh_start = Board::new();
        assert_eq!(
            board.grid, fresh_start.grid,
            "Full unwinding failed to restore initial grid at game {game_idx}"
        );
        assert_eq!(
            board.bitboards, fresh_start.bitboards,
            "Full unwinding failed to restore bitboards at game {game_idx}"
        );
        assert_eq!(
            board.zobrist_hash, fresh_start.zobrist_hash,
            "Full unwinding failed to restore initial Zobrist hash at game {game_idx}"
        );
        assert_eq!(
            board.nnue_acc.red, fresh_start.nnue_acc.red,
            "Full unwinding failed to restore initial NNUE red at game {game_idx}"
        );
        assert_eq!(
            board.nnue_acc.black, fresh_start.nnue_acc.black,
            "Full unwinding failed to restore initial NNUE black at game {game_idx}"
        );
    }
}

#[test]
fn prop_incremental_nnue_vs_refresh() {
    let seed = get_seed() ^ 0xAA_55_AA_55;
    let num_games = get_game_count(150);
    println!("Running prop_incremental_nnue_vs_refresh with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let mg = MoveGenerator::new();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let max_plies = rng.gen_range(20..=60);

        for ply in 0..max_plies {
            let Some((_mv, _captured)) =
                play_random_legal_move(&mut board, &mut turn, &mg, &mut rng)
            else {
                break;
            };

            // Verify incrementally updated accumulator matches fresh refresh
            let mut fresh = board.clone();
            fresh.refresh_nnue();

            assert_eq!(
                board.nnue_acc.red, fresh.nnue_acc.red,
                "Incremental NNUE red differs from fresh refresh at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                board.nnue_acc.black, fresh.nnue_acc.black,
                "Incremental NNUE black differs from fresh refresh at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                board.nnue_acc.psqt_red, fresh.nnue_acc.psqt_red,
                "Incremental NNUE psqt_red differs at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                board.nnue_acc.psqt_black, fresh.nnue_acc.psqt_black,
                "Incremental NNUE psqt_black differs at game {game_idx}, ply {ply}"
            );
        }
    }
}

#[test]
fn prop_incremental_zobrist_vs_scratch() {
    let seed = get_seed() ^ 0x55_AA_55_AA;
    let num_games = get_game_count(150);
    println!("Running prop_incremental_zobrist_vs_scratch with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let mg = MoveGenerator::new();
    let keys = ZobristKeys::get();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let max_plies = rng.gen_range(20..=60);

        for ply in 0..max_plies {
            let Some((_mv, _captured)) =
                play_random_legal_move(&mut board, &mut turn, &mg, &mut rng)
            else {
                break;
            };

            let piece_hash = board.calculate_initial_hash();
            let expected_hash = if turn == Color::Black {
                piece_hash ^ keys.side_key
            } else {
                piece_hash
            };

            assert_eq!(
                board.zobrist_hash, expected_hash,
                "Incremental Zobrist hash mismatch at game {game_idx}, ply {ply}"
            );
        }
    }
}

#[test]
fn prop_fen_roundtrip_equivalence() {
    let seed = get_seed() ^ 0x12_34_56_78;
    let num_games = get_game_count(120);
    println!("Running prop_fen_roundtrip_equivalence with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let mg = MoveGenerator::new();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let max_plies = rng.gen_range(10..=45);

        for ply in 0..max_plies {
            let fen = board.to_fen_string(turn);
            let (parsed_board, parsed_turn) =
                Board::from_fen(&fen).expect("Exported FEN must parse cleanly");

            assert_eq!(
                parsed_turn, turn,
                "Parsed turn mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.grid, board.grid,
                "Parsed grid mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.bitboards, board.bitboards,
                "Parsed bitboards mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.occupied, board.occupied,
                "Parsed occupied mask mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.occupied_rows, board.occupied_rows,
                "Parsed occupied_rows mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.occupied_cols, board.occupied_cols,
                "Parsed occupied_cols mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.zobrist_hash, board.zobrist_hash,
                "Parsed Zobrist hash mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.nnue_acc.red, board.nnue_acc.red,
                "Parsed NNUE red mismatch at game {game_idx}, ply {ply}"
            );
            assert_eq!(
                parsed_board.nnue_acc.black, board.nnue_acc.black,
                "Parsed NNUE black mismatch at game {game_idx}, ply {ply}"
            );

            if play_random_legal_move(&mut board, &mut turn, &mg, &mut rng).is_none() {
                break;
            }
        }
    }
}

#[test]
fn prop_movegen_matches_engine_movegen() {
    let seed = get_seed() ^ 0x99_88_77_66;
    let num_games = get_game_count(100);
    println!("Running prop_movegen_matches_engine_movegen with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let logic_mg = MoveGenerator::new();
    let engine_mg = EngineMoveGen::new();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let max_plies = rng.gen_range(10..=40);

        for ply in 0..max_plies {
            // 1. Logic Move Generator strictly legal moves
            let mut logic_moves: Vec<(u8, u8, u8, u8)> = logic_mg
                .generate_moves(&board, turn)
                .into_iter()
                .map(|m| (m.from_row, m.from_col, m.to_row, m.to_col))
                .collect();
            logic_moves.sort_unstable();

            // 2. Engine Move Generator pseudo-legal moves filtered for legality
            let engine_list = engine_mg.generate_moves(&board, turn, None);
            let mut engine_moves: Vec<(u8, u8, u8, u8)> = Vec::with_capacity(engine_list.len());

            for &mv in engine_list.iter() {
                let mut test_board = board.clone();
                let Some((piece, _captured, from_sq, _to_sq)) =
                    test_board.apply_move_board_only(&mv)
                else {
                    continue;
                };

                let is_flying =
                    if could_cause_flying_general(&test_board, piece.piece_type, from_sq) {
                        is_flying_general(&test_board)
                    } else {
                        false
                    };

                if !is_in_check(&test_board, turn) && !is_flying {
                    engine_moves.push((mv.from_row, mv.from_col, mv.to_row, mv.to_col));
                }
            }
            engine_moves.sort_unstable();

            assert_eq!(
                logic_moves, engine_moves,
                "Move lists differ between Logic MoveGen and Engine MoveGen at game {game_idx}, ply {ply}"
            );

            if play_random_legal_move(&mut board, &mut turn, &logic_mg, &mut rng).is_none() {
                break;
            }
        }
    }
}

#[test]
fn prop_generated_moves_strictly_safe() {
    let seed = get_seed() ^ 0x33_44_55_66;
    let num_games = get_game_count(100);
    println!("Running prop_generated_moves_strictly_safe with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);
    let mg = MoveGenerator::new();

    for game_idx in 0..num_games {
        let mut board = Board::new();
        let mut turn = Color::Red;
        let max_plies = rng.gen_range(10..=35);

        for ply in 0..max_plies {
            let moves = mg.generate_moves(&board, turn);

            for mv in &moves {
                let mut simulated = board.clone();
                let to_coord =
                    BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize).expect("Valid");
                let _captured = simulated.get_piece(to_coord);
                simulated.apply_move(mv, turn);

                assert!(
                    !is_in_check(&simulated, turn),
                    "Legal move left own king in check! Game {game_idx}, ply {ply}, move: {mv:?}"
                );
                assert!(
                    !is_flying_general(&simulated),
                    "Legal move allowed flying generals! Game {game_idx}, ply {ply}, move: {mv:?}"
                );
            }

            if play_random_legal_move(&mut board, &mut turn, &mg, &mut rng).is_none() {
                break;
            }
        }
    }
}

#[test]
fn prop_uci_move_roundtrip() {
    // Tests all 8,100 combinations of coordinates on the 10x9 board
    for from_row in 0..10u8 {
        for from_col in 0..9u8 {
            for to_row in 0..10u8 {
                for to_col in 0..9u8 {
                    let mv = Move {
                        from_row,
                        from_col,
                        to_row,
                        to_col,
                        score: 0,
                    };
                    let uci = mv.to_uci();
                    assert_eq!(uci.len(), 4, "UCI string must be 4 characters");

                    let parsed = Move::from_uci(&uci)
                        .expect("Valid board coordinates must parse back from UCI");
                    assert_eq!(parsed.from_row, mv.from_row);
                    assert_eq!(parsed.from_col, mv.from_col);
                    assert_eq!(parsed.to_row, mv.to_row);
                    assert_eq!(parsed.to_col, mv.to_col);
                }
            }
        }
    }
}

#[test]
fn prop_gamestate_undo_stack_invariance() {
    let seed = get_seed() ^ 0xDE_AD_BE_EF;
    let num_games = get_game_count(100);
    println!("Running prop_gamestate_undo_stack_invariance with seed {seed:#x}, {num_games} games");

    let mut rng = StdRng::seed_from_u64(seed);

    for game_idx in 0..num_games {
        let mut state = GameState::new();
        let initial_grid = state.board.grid;
        let initial_hash = state.board.zobrist_hash;
        let initial_turn = state.turn;

        let max_moves = rng.gen_range(10..=40);
        let mut moves_made = 0;

        for _ in 0..max_moves {
            let legal_moves = state.get_strictly_legal_moves();
            if legal_moves.is_empty() {
                break;
            }
            let mv = legal_moves[rng.gen_range(0..legal_moves.len())];
            let from =
                BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize).expect("Valid");
            let to = BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize).expect("Valid");

            if state.make_move(from, to).is_ok() {
                moves_made += 1;
            } else {
                break;
            }
        }

        assert_eq!(
            state.history.len(),
            moves_made,
            "History length mismatch at game {game_idx}"
        );

        // Undo all moves made
        for _ in 0..moves_made {
            assert!(
                state.undo_move(),
                "undo_move failed unexpectedly at game {game_idx}"
            );
        }

        assert!(
            state.history.is_empty(),
            "History should be empty after unwinding at game {game_idx}"
        );
        assert_eq!(
            state.turn, initial_turn,
            "Turn mismatch after unwinding at game {game_idx}"
        );
        assert_eq!(
            state.board.grid, initial_grid,
            "Board grid mismatch after unwinding at game {game_idx}"
        );
        assert_eq!(
            state.board.zobrist_hash, initial_hash,
            "Zobrist hash mismatch after unwinding at game {game_idx}"
        );
    }
}
