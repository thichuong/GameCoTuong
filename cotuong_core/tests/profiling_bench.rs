#![allow(clippy::all, clippy::pedantic, clippy::unwrap_used)]

use cotuong_core::engine::config::EngineConfig;
use cotuong_core::engine::movegen::EngineMoveGen;
use cotuong_core::engine::nnue::{evaluate_nnue, NnueWeights};
use cotuong_core::engine::search::AlphaBetaEngine;
use cotuong_core::engine::{SearchLimit, Searcher};
use cotuong_core::logic::board::{Board, Color, Piece, PieceType};
use cotuong_core::logic::game::GameState;
use std::sync::Arc;
use std::time::Instant;

#[test]
fn profile_standalone_nnue_eval() {
    println!("\n=== PROFILE: Standalone NNUE Evaluation (SIMD Accelerated) ===");
    let board = Board::new();

    // Warmup
    #[cfg(debug_assertions)]
    let warmup_count = 1_000;
    #[cfg(not(debug_assertions))]
    let warmup_count = 10_000;

    for _ in 0..warmup_count {
        std::hint::black_box(evaluate_nnue(&board));
    }

    #[cfg(debug_assertions)]
    let iterations = 10_000;
    #[cfg(not(debug_assertions))]
    let iterations = 500_000;

    let start = Instant::now();
    let mut dummy_sum: i64 = 0;
    for _ in 0..iterations {
        dummy_sum += std::hint::black_box(evaluate_nnue(&board)) as i64;
    }
    let duration = start.elapsed();
    let evals_per_sec = iterations as f64 / duration.as_secs_f64();

    println!("Total iterations: {}", iterations);
    println!("Time taken: {:?}", duration);
    println!("Standalone NNUE NPS (evals/sec): {:.0}", evals_per_sec);
    println!(
        "Time per single eval: {:.2} ns",
        (duration.as_nanos() as f64) / iterations as f64
    );
    println!("Dummy sum: {}", dummy_sum);
}

#[test]
fn profile_accumulator_update() {
    println!("\n=== PROFILE: Accumulator Update ===");
    let mut board = Board::new();
    let weights = NnueWeights::get();

    let from_sq = 19; // Cannon
    let to_sq = 22; // Middle file
    let piece = Piece {
        piece_type: PieceType::Cannon,
        color: Color::Red,
    };

    // Warmup
    for _ in 0..10_000 {
        board.update_nnue_for_move_with_weights(from_sq, to_sq, piece, None, weights);
        board.update_nnue_for_move_with_weights(to_sq, from_sq, piece, None, weights);
    }

    #[cfg(debug_assertions)]
    let iterations = 50_000;
    #[cfg(not(debug_assertions))]
    let iterations = 1_000_000;

    let start = Instant::now();
    for _ in 0..iterations {
        board.update_nnue_for_move_with_weights(from_sq, to_sq, piece, None, weights);
    }
    let duration = start.elapsed();
    let updates_per_sec = iterations as f64 / duration.as_secs_f64();

    println!("Total iterations: {}", iterations);
    println!("Time taken: {:?}", duration);
    println!("Accumulator updates/sec: {:.0}", updates_per_sec);
    println!(
        "Time per single acc update: {:.2} ns",
        (duration.as_nanos() as f64) / iterations as f64
    );
}

#[test]
fn profile_movegen_speed() {
    println!("\n=== PROFILE: Move Generation Speed ===");
    let board = Board::new();
    let movegen = EngineMoveGen::new();

    let iterations = 100_000;
    let start = Instant::now();
    let mut total_moves = 0;
    for _ in 0..iterations {
        let moves = movegen.generate_moves(&board, Color::Red, None);
        total_moves += moves.len();
    }
    let duration = start.elapsed();
    let gen_per_sec = (iterations as f64) / duration.as_secs_f64();

    println!("Movegen runs/sec: {:.0}", gen_per_sec);
    println!(
        "Time per movegen: {:.2} ns",
        (duration.as_nanos() as f64) / iterations as f64
    );
    println!(
        "Average legal moves generated: {:.1}",
        total_moves as f64 / iterations as f64
    );
}

#[test]
fn profile_search_nnue_performance() {
    println!("\n=== PROFILE: Search NNUE Performance (Opening & Midgame) ===");
    let config = Arc::new(EngineConfig::default());
    let mut engine = AlphaBetaEngine::new(config);
    let game_state = GameState::new();

    // Warmup
    engine.search(&game_state, SearchLimit::Depth(2), &[]);

    // Search Depth 6
    let start = Instant::now();
    let res = engine.search(&game_state, SearchLimit::Depth(6), &[]);
    let duration = start.elapsed();

    let (mv, stats) = res.expect("Search must return valid move");
    let nps = (stats.nodes as f64 / duration.as_secs_f64()) as u64;

    println!(
        "Depth: {}, Best Move: {}, Nodes: {}, Time: {:?}, NPS: {}",
        stats.depth,
        mv.to_uci(),
        stats.nodes,
        duration,
        nps
    );
    assert!(nps > 0);
}
