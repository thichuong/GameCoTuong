#![no_main]

use cotuong_core::engine::Move;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Ok(mv) = Move::from_uci(s) {
            assert!(mv.from_row <= 9);
            assert!(mv.from_col <= 8);
            assert!(mv.to_row <= 9);
            assert!(mv.to_col <= 8);
            let _ = mv.to_uci();
        }
    }
});
