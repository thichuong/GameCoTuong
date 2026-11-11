#![no_main]

use cotuong_core::logic::board::Board;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok((mut board, turn)) = Board::from_fen(s) {
            board.refresh_nnue();
            let _ = board.calculate_initial_hash();
            let _ = board.to_fen_string(turn);
        }
    }
});
