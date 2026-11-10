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

#[test]
fn profile_perft_speed() {
    use cotuong_core::engine::perft;

    println!("\n=== PROFILE: Perft Speed (MoveGen + Make/Undo + Legal Validation) ===");
    let mut board = Board::new();

    for d in 1..=4 {
        let start = Instant::now();
        let nodes = perft(&mut board, Color::Red, d);
        let duration = start.elapsed();
        let nps = (nodes as f64 / duration.as_secs_f64()) as u64;
        let time_per_node_ns = (duration.as_nanos() as f64) / (nodes as f64);

        println!(
            "Depth {}: Nodes = {:>9}, Time = {:>10.3?}, NPS = {:>10}, Time/Node = {:>6.2} ns",
            d, nodes, duration, nps, time_per_node_ns
        );
        assert!(nodes > 0);
    }
}

#[test]
fn profile_search_no_eval_vs_nnue() {
    println!("\n================================================================================");
    println!("=== PROFILE: Search Node Traversal: Standard NNUE vs No-Eval (Zero-Eval)     ===");
    println!("================================================================================");

    let game_state = GameState::new();

    for depth in [5u8, 6u8] {
        println!("\n--- Comparing Search at Depth {} ---", depth);

        // 1. With Standard NNUE Eval
        let config_nnue = Arc::new(EngineConfig::default());
        let mut engine_nnue = AlphaBetaEngine::new(config_nnue);
        engine_nnue.search(&game_state, SearchLimit::Depth(2), &[]); // Warmup

        let start_nnue = Instant::now();
        let res_nnue = engine_nnue.search(&game_state, SearchLimit::Depth(depth), &[]);
        let dur_nnue = start_nnue.elapsed();
        let (mv_nnue, stats_nnue) = res_nnue.expect("NNUE search must succeed");
        let nps_nnue = (stats_nnue.nodes as f64 / dur_nnue.as_secs_f64()) as u64;
        let ns_per_node_nnue = (dur_nnue.as_nanos() as f64) / (stats_nnue.nodes as f64);

        // 2. With No-Eval (Zero Eval, bypass NNUE accumulator & forward pass)
        let config_no_eval = Arc::new(EngineConfig {
            nnue_weight: 0,
            ..Default::default()
        });
        let mut engine_no_eval = AlphaBetaEngine::new(config_no_eval);
        engine_no_eval.search(&game_state, SearchLimit::Depth(2), &[]); // Warmup

        let start_no_eval = Instant::now();
        let res_no_eval = engine_no_eval.search(&game_state, SearchLimit::Depth(depth), &[]);
        let dur_no_eval = start_no_eval.elapsed();
        let (mv_no_eval, stats_no_eval) = res_no_eval.expect("No-eval search must succeed");
        let nps_no_eval = (stats_no_eval.nodes as f64 / dur_no_eval.as_secs_f64()) as u64;
        let ns_per_node_no_eval = (dur_no_eval.as_nanos() as f64) / (stats_no_eval.nodes as f64);

        println!(
            "{:<18} | Nodes: {:>7} | Time: {:>9.3?} | NPS: {:>9} | Time/Node: {:>7.2} ns | BestMove: {}",
            "With NNUE Eval",
            stats_nnue.nodes,
            dur_nnue,
            nps_nnue,
            ns_per_node_nnue,
            mv_nnue.to_uci()
        );
        println!(
            "{:<18} | Nodes: {:>7} | Time: {:>9.3?} | NPS: {:>9} | Time/Node: {:>7.2} ns | BestMove: {}",
            "Without Eval",
            stats_no_eval.nodes,
            dur_no_eval,
            nps_no_eval,
            ns_per_node_no_eval,
            mv_no_eval.to_uci()
        );

        let speedup_ratio = (nps_no_eval as f64) / (nps_nnue as f64);
        let eval_overhead_ns = (ns_per_node_nnue - ns_per_node_no_eval).max(0.0);
        let eval_pct = (eval_overhead_ns / ns_per_node_nnue) * 100.0;

        println!(">> Raw NPS Speedup without Eval: {:.2}x", speedup_ratio);
        println!(
            ">> Estimated Evaluation Cost per Node: {:.2} ns ({:.1}% of search time)",
            eval_overhead_ns, eval_pct
        );
    }
}
