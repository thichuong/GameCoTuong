//! In-tree Fuzzing test suite for cotuong_core.
//!
//! Stress-tests parsers, state deserialization, board operations, and state machine transitions
//! against tens of thousands of malformed, boundary-case, and random inputs to ensure 100%
//! panic-free execution and crash-resilience:
//! 1. `fuzz_fen_parser`: Arbitrary UTF-8 and mutated strings into `Board::from_fen`
//! 2. `fuzz_uci_parser`: Arbitrary string lengths and characters into `Move::from_uci`
//! 3. `fuzz_json_deserializer`: Corrupted JSON into `GameState` and `EngineConfig`
//! 4. `fuzz_board_coordinate_and_operations`: Extreme coordinates (`usize::MAX`, out of bounds)
//! 5. `fuzz_game_random_operation_stream`: Random command stream on `GameState`

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::Move;
use cotuong_core::logic::board::{Board, BoardCoordinate, Color, Piece, PieceType};
use cotuong_core::logic::game::GameState;
use cotuong_core::logic::rules::is_valid_move;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

fn get_seed() -> u64 {
    std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0xF0_02_20_26_AA)
}

fn get_iteration_count(default: usize) -> usize {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[test]
fn fuzz_fen_parser() {
    let seed = get_seed();
    let num_cases = get_iteration_count(10_000);
    println!("Running fuzz_fen_parser with seed {seed:#x}, {num_cases} iterations");

    let mut rng = StdRng::seed_from_u64(seed);

    let seed_fens = [
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w",
        "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR b",
        "4k4/9/9/9/9/9/9/9/9/4K4 w",
        "3ak4/9/9/9/9/9/9/9/9/3AK4 b",
        "2b1k1b2/9/9/9/9/9/9/9/9/2B1K1B2 w",
    ];

    let chars_pool = [
        'r', 'n', 'b', 'a', 'k', 'c', 'p', 'R', 'N', 'B', 'A', 'K', 'C', 'P', '/', ' ', 'w', 'b',
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'x', 'Z', '?', '\0', '\n', '\t', '-',
        '+', ':', ';',
    ];

    for i in 0..num_cases {
        let strategy = i % 5;
        let candidate: String = match strategy {
            // 0: Mutation of standard FENs
            0 => {
                let base = seed_fens[rng.gen_range(0..seed_fens.len())];
                let mut chars: Vec<char> = base.chars().collect();
                let mutations = rng.gen_range(1..=5);
                for _ in 0..mutations {
                    let idx = rng.gen_range(0..chars.len());
                    let mut_type = rng.gen_range(0..3);
                    match mut_type {
                        0 => chars[idx] = chars_pool[rng.gen_range(0..chars_pool.len())],
                        1 => chars.insert(idx, chars_pool[rng.gen_range(0..chars_pool.len())]),
                        _ => {
                            if chars.len() > 1 {
                                chars.remove(idx);
                            }
                        }
                    }
                }
                chars.into_iter().collect()
            }
            // 1: Random ASCII string from chars pool
            1 => {
                let len = rng.gen_range(0..=80);
                (0..len)
                    .map(|_| chars_pool[rng.gen_range(0..chars_pool.len())])
                    .collect()
            }
            // 2: Arbitrary random utf-8 byte stream
            2 => {
                let len = rng.gen_range(0..=128);
                let bytes: Vec<u8> = (0..len).map(|_| rng.gen::<u8>()).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            }
            // 3: Edge cases (slashes, digits, whitespace)
            3 => {
                let parts = rng.gen_range(0..=15);
                let mut fen = String::new();
                for p in 0..parts {
                    if p > 0 {
                        fen.push('/');
                    }
                    let count: usize = rng.gen_range(0..=12);
                    fen.push_str(&count.to_string());
                }
                fen.push(' ');
                fen.push(if rng.gen_bool(0.5) { 'w' } else { 'b' });
                fen
            }
            // 4: Empty, single char, or huge whitespace
            _ => {
                let whitespace_len = rng.gen_range(0..=50);
                let mut s = " ".repeat(whitespace_len);
                if rng.gen_bool(0.5) {
                    s.push_str("rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w");
                }
                s
            }
        };

        // Must never panic on any candidate string
        if let Ok((mut board, turn)) = Board::from_fen(&candidate) {
            // If FEN parsed successfully, internal methods must also execute cleanly without panic
            board.refresh_nnue();
            let _ = board.calculate_initial_hash();
            let exported = board.to_fen_string(turn);
            assert!(
                !exported.is_empty(),
                "Exported FEN cannot be empty for parsed board"
            );
        }
    }
}

#[test]
fn fuzz_uci_parser() {
    let seed = get_seed() ^ 0x44_33_22_11;
    let num_cases = get_iteration_count(10_000);
    println!("Running fuzz_uci_parser with seed {seed:#x}, {num_cases} iterations");

    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..num_cases {
        let len = rng.gen_range(0..=16);
        let s: String = match rng.gen_range(0..3) {
            // Coordinate-like characters near standard bounds
            0 => (0..len)
                .map(|_| match rng.gen_range(0..4) {
                    0 => (b'a' + rng.gen_range(0..15)) as char,
                    1 => (b'0' + rng.gen_range(0..15)) as char,
                    2 => rng.gen_range('A'..='Z'),
                    _ => rng.gen_range('\0'..='/'),
                })
                .collect(),
            // Random ASCII
            1 => (0..len)
                .map(|_| rng.gen_range(32u8..=126) as char)
                .collect(),
            // Random binary / UTF-8
            _ => {
                let bytes: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            }
        };

        // Move::from_uci must never panic
        if let Ok(mv) = Move::from_uci(&s) {
            // If parsed as valid UCI, coordinates must be strictly within Xiangqi board bounds
            assert!(
                mv.from_row <= 9,
                "from_row exceeds 9 on valid UCI parse: {s}"
            );
            assert!(
                mv.from_col <= 8,
                "from_col exceeds 8 on valid UCI parse: {s}"
            );
            assert!(mv.to_row <= 9, "to_row exceeds 9 on valid UCI parse: {s}");
            assert!(mv.to_col <= 8, "to_col exceeds 8 on valid UCI parse: {s}");

            let serialized = mv.to_uci();
            assert_eq!(serialized.len(), 4);
        }
    }
}

#[test]
fn fuzz_json_deserializer() {
    let seed = get_seed() ^ 0x77_66_55_44;
    let num_cases = get_iteration_count(5_000);
    println!("Running fuzz_json_deserializer with seed {seed:#x}, {num_cases} iterations");

    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..num_cases {
        let json_str: String = match rng.gen_range(0..4) {
            // JSON template with mutated numbers/types
            0 => {
                let random_num: i64 = rng.gen();
                format!(
                    r#"{{"board":{{"bitboards":[{},0,0,0,0,0,0,0,0,0,0,0,0,0],"occupied":{},"grid":[]}},"turn":"Red","status":"Playing","history":[]}}"#,
                    random_num, random_num
                )
            }
            // EngineConfig mutation
            1 => {
                let mate_score: i64 = rng.gen();
                let contempt: i64 = rng.gen();
                format!(
                    r#"{{"mate_score":{},"contempt":{},"enable_lmr":{}}}"#,
                    mate_score,
                    contempt,
                    rng.gen_bool(0.5)
                )
            }
            // Truncated or malformed JSON
            2 => {
                let valid = r#"{"mate_score":30000,"draw_score":0,"book_enabled":true}"#;
                let cut = rng.gen_range(0..valid.len());
                valid.get(..cut).unwrap_or("").to_string()
            }
            // Random string / noise
            _ => {
                let len = rng.gen_range(0..=60);
                (0..len)
                    .map(|_| rng.gen_range(b' '..=b'}') as char)
                    .collect()
            }
        };

        // EngineConfig deserialization must never panic
        let _ = serde_json::from_str::<EngineConfig>(&json_str);

        // GameState deserialization must never panic
        if let Ok(mut state) = serde_json::from_str::<GameState>(&json_str) {
            // Even with arbitrary deserialized fields, refresh_nnue must handle it safely
            state.board.refresh_nnue();
        }
    }
}

#[test]
fn fuzz_board_coordinate_and_operations() {
    let seed = get_seed() ^ 0xEE_DD_CC_BB;
    let num_cases = get_iteration_count(10_000);
    println!(
        "Running fuzz_board_coordinate_and_operations with seed {seed:#x}, {num_cases} iterations"
    );

    let mut rng = StdRng::seed_from_u64(seed);
    let mut board = Board::new();

    let extreme_coords = [0, 1, 8, 9, 10, 11, 89, 90, 100, usize::MAX - 1, usize::MAX];

    for _ in 0..num_cases {
        let r = if rng.gen_bool(0.3) {
            extreme_coords[rng.gen_range(0..extreme_coords.len())]
        } else {
            rng.gen_range(0..20)
        };
        let c = if rng.gen_bool(0.3) {
            extreme_coords[rng.gen_range(0..extreme_coords.len())]
        } else {
            rng.gen_range(0..20)
        };

        let coord = BoardCoordinate::new(r, c);
        if r < 10 && c < 9 {
            assert!(coord.is_some());
            let c_val = coord.expect("Checked bounds");
            assert_eq!(c_val.index(), r * 9 + c);
            let _ = board.get_piece(c_val);
        } else {
            assert!(coord.is_none());
        }

        let mv = Move {
            from_row: (r % 256) as u8,
            from_col: (c % 256) as u8,
            to_row: (r.wrapping_add(1) % 256) as u8,
            to_col: (c.wrapping_add(1) % 256) as u8,
            score: 0,
        };

        // Board operations must handle any coordinates without panics
        if let Some((piece, captured, from_sq, to_sq)) = board.apply_move_board_only(&mv) {
            board.update_nnue_for_move(from_sq, to_sq, piece, captured);
            board.undo_move_board_only(&mv, piece, captured);
        }

        // is_valid_move must handle any coordinate pair safely
        if let (Some(from), Some(to)) = (BoardCoordinate::new(r, c), BoardCoordinate::new(c, r)) {
            let _ = is_valid_move(&board, from, to, Color::Red);
            let _ = is_valid_move(&board, from, to, Color::Black);
        }
    }
}

#[test]
fn fuzz_game_random_operation_stream() {
    let seed = get_seed() ^ 0x11_22_33_44;
    let num_games = get_iteration_count(1_000);
    println!("Running fuzz_game_random_operation_stream with seed {seed:#x}, {num_games} streams");

    let mut rng = StdRng::seed_from_u64(seed);

    for _ in 0..num_games {
        let mut game = GameState::new();
        let ops_count = rng.gen_range(20..=60);

        for _ in 0..ops_count {
            let op_type = rng.gen_range(0..7);
            match op_type {
                // 0: Make move with arbitrary coordinates
                0 => {
                    let fr = rng.gen_range(0..15);
                    let fc = rng.gen_range(0..15);
                    let tr = rng.gen_range(0..15);
                    let tc = rng.gen_range(0..15);

                    if let (Some(from), Some(to)) =
                        (BoardCoordinate::new(fr, fc), BoardCoordinate::new(tr, tc))
                    {
                        let _ = game.make_move(from, to);
                    }
                }
                // 1: Make strictly legal move if available
                1 => {
                    let legal = game.get_strictly_legal_moves();
                    if !legal.is_empty() {
                        let mv = legal[rng.gen_range(0..legal.len())];
                        if let (Some(from), Some(to)) = (
                            BoardCoordinate::new(mv.from_row as usize, mv.from_col as usize),
                            BoardCoordinate::new(mv.to_row as usize, mv.to_col as usize),
                        ) {
                            let _ = game.make_move(from, to);
                        }
                    }
                }
                // 2: Undo move
                2 => {
                    let _ = game.undo_move();
                }
                // 3: Status check
                3 => {
                    let _ = game.status;
                }
                // 4: Export FEN
                4 => {
                    let _ = game.board.to_fen_string(game.turn);
                }
                // 5: Direct piece mutation
                5 => {
                    let r = rng.gen_range(0..10);
                    let c = rng.gen_range(0..9);
                    if let Some(coord) = BoardCoordinate::new(r, c) {
                        let piece = if rng.gen_bool(0.5) {
                            Some(Piece {
                                piece_type: PieceType::General,
                                color: Color::Red,
                            })
                        } else {
                            None
                        };
                        game.board.set_piece(coord, piece);
                    }
                }
                // 6: Reset game
                _ => {
                    game = GameState::new();
                }
            }
        }
    }
}
