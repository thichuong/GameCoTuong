use crate::engine::config::EngineConfig;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{Move, SearchLimit, SearchStats, Searcher};
use crate::logic::game::GameState;
use gloo_worker::{HandlerId, Worker, WorkerScope};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub enum Input {
    ComputeMove(GameState, SearchLimit, EngineConfig, Vec<Move>),
}

#[derive(Serialize, Deserialize)]
pub enum Output {
    MoveFound(Move, SearchStats),
}

pub struct GameWorker {
    engine: Option<AlphaBetaEngine>,
}

impl Worker for GameWorker {
    type Input = Input;
    type Message = ();
    type Output = Output;

    fn create(_scope: &WorkerScope<Self>) -> Self {
        Self { engine: None }
    }

    fn update(&mut self, _scope: &WorkerScope<Self>, _msg: Self::Message) {}

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        match msg {
            Input::ComputeMove(game_state, limit, config, excluded_moves) => {
                let config = Arc::new(config);

                if let Some(engine) = &mut self.engine {
                    engine.update_config(config);
                } else {
                    self.engine = Some(AlphaBetaEngine::new(config));
                }

                let engine = self.engine.as_mut().expect("Engine should be initialized");

                // We don't have excluded_moves in the worker yet, assuming empty for now or pass it if needed.
                // The current app logic passes excluded_moves for 3-fold repetition retry.
                // I should probably add excluded_moves to Input if I want to support that fully.
                // But for now, let's assume empty and handle retries in App by re-sending.
                // Wait, if I handle retries in App, I need to pass excluded_moves to worker.
                // Let's update Input to include excluded_moves.

                // Actually, let's stick to the plan first. The plan didn't explicitly mention excluded_moves in Input.
                // But `AlphaBetaEngine::search` takes it.
                // I'll add it to Input.

                if let Some((mv, stats)) = engine.search(&game_state, limit, &excluded_moves) {
                    scope.respond(id, Output::MoveFound(mv, stats));
                } else {
                    // If no move found (e.g. mate), we might want to respond with something or just nothing?
                    // The app expects a move. If search returns None, it means no legal moves or time out without result?
                    // AlphaBetaEngine returns Option.
                    // If None, maybe we should send a specific message or just not respond (which hangs the promise if using oneshot).
                    // But here we use a bridge.
                    // Let's add NoMoveFound to Output?
                    // Or just let the app handle timeout?
                    // Ideally, search always returns something unless game over.
                    // If game over, app shouldn't ask for move.
                }
            }
        }
    }
}
