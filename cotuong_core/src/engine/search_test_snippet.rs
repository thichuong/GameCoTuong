#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::board::{Board, BoardCoordinate, PieceType};
    use crate::logic::game::GameState;

    #[test]
    fn test_mate_score_priority() {
        let config = Arc::new(EngineConfig::default());
        let mut engine = AlphaBetaEngine::new(config.clone());

        // We can't easily mock the board to force AlphaBeta to specific depths without complex setup.
        // But we can check if the RETURN VALUE logic aligns with our formula.
        // Loop over depths and check the returned score for a theoretical mate.

        // Formula: -MATE + ply
        // Winning: +MATE - ply (Since alpha_beta returns relative score, caller negates it)
        // If I am winning at ply 3: Child returns -(MATE - 3) [Losing for opponent]. I see (MATE - 3).
        // If I am winning at ply 5: Child returns -(MATE - 5). I see (MATE - 5).
        // (MATE - 3) > (MATE - 5). Correct.

        // If I am losing at ply 3: Self returns -(MATE - 3). Score is -(MATE - 3).
        // If I am losing at ply 5: Self returns -(MATE - 5). Score is -(MATE - 5).
        // -(MATE - 5) > -(MATE - 3).
        // e.g. -29995 > -29997.
        // So Losing at ply 5 is BETTER (higher score) than Losing at ply 3.
        // This means the engine will try to delay mate. CORRECT.

        // Let's create a scenario where we can verify this behavior via search.
        // Position:
        // Red Chariot at (9,0). Red General at (0,4).
        // Black General at (0, 5). Black has no defense.
        // (This is impossible board but simplifies).
        // Let's use a real mate puzzle.

        // Actually, ensuring the FORMULA in the code is correct is step 1.
        // I changed `(10 - depth)` to `ply`.
        // Let's verify via a small test calling alpha_beta on a forced mate position?
        // `alpha_beta` is private. But we are in `mod tests` inside `search.rs`.
        // Wait, `search.rs` does not have `mod tests` at the bottom currently?
        // I need to add it.
    }
}
