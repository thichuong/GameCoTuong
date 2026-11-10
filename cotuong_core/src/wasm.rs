// WASM interface for cotuong_core
// Provides JavaScript bindings for board state, move generation, and AI search

use crate::engine::config::EngineConfig;
use crate::engine::search::AlphaBetaEngine;
use crate::engine::{SearchLimit, Searcher};
use crate::logic::board::{Board, BoardCoordinate, Color, Piece, PieceType};
use crate::logic::game::{GameState, GameStatus, MoveRecord};
use crate::logic::opening;
use crate::logic::rules::is_in_check;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
struct PieceDto {
    #[serde(rename = "type")]
    piece_type: String,
    color: String,
}

impl PieceDto {
    fn from_piece(p: Piece) -> Self {
        let piece_type = match p.piece_type {
            PieceType::General => "General",
            PieceType::Advisor => "Advisor",
            PieceType::Elephant => "Elephant",
            PieceType::Horse => "Horse",
            PieceType::Chariot => "Chariot",
            PieceType::Cannon => "Cannon",
            PieceType::Soldier => "Soldier",
        };
        let color = match p.color {
            Color::Red => "red",
            Color::Black => "black",
        };
        Self {
            piece_type: piece_type.to_string(),
            color: color.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SquareDto {
    row: usize,
    col: usize,
    piece: Option<PieceDto>,
}

#[derive(Serialize, Deserialize)]
struct StatusDto {
    status: String,
    winner: Option<String>,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct MoveResultDto {
    success: bool,
    captured: Option<PieceDto>,
    in_check: bool,
}

#[derive(Serialize, Deserialize)]
struct HistoryRecordDto {
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
    piece: PieceDto,
    captured: Option<PieceDto>,
    note: Option<String>,
}

impl HistoryRecordDto {
    fn from_record(r: &MoveRecord) -> Self {
        Self {
            from_row: r.from.row,
            from_col: r.from.col,
            to_row: r.to.row,
            to_col: r.to.col,
            piece: PieceDto::from_piece(r.piece),
            captured: r.captured.map(PieceDto::from_piece),
            note: r.note.as_ref().map(ToString::to_string),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct MoveDto {
    from_row: usize,
    from_col: usize,
    to_row: usize,
    to_col: usize,
}

#[derive(Serialize, Deserialize)]
struct SearchStatsDto {
    depth: u8,
    nodes: u32,
    time_ms: u64,
}

#[derive(Serialize, Deserialize)]
struct SearchResultDto {
    r#move: MoveDto,
    stats: SearchStatsDto,
}

/// WASM-compatible wrapper around GameState
#[wasm_bindgen]
pub struct WasmGameState {
    inner: GameState,
}

#[wasm_bindgen]
impl WasmGameState {
    /// Create a new game in the starting position
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: GameState::new(),
        }
    }

    /// Load game state from a FEN string
    pub fn from_fen(fen: &str) -> Result<WasmGameState, JsValue> {
        let (board, turn) =
            Board::from_fen(fen).map_err(|e| JsValue::from_str(&format!("Invalid FEN: {e}")))?;
        let mut state = GameState::new();
        state.initial_hash = board.zobrist_hash;
        state.board = board;
        state.turn = turn;
        Ok(Self { inner: state })
    }

    /// Export current board position to FEN
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.inner.board.to_fen_string(self.inner.turn)
    }

    /// Current side to move ("red" or "black")
    #[must_use]
    pub fn turn(&self) -> String {
        match self.inner.turn {
            Color::Red => "red".to_string(),
            Color::Black => "black".to_string(),
        }
    }

    /// Whether current side to move is in check
    #[must_use]
    pub fn is_in_check(&self) -> bool {
        is_in_check(&self.inner.board, self.inner.turn)
    }

    /// Return game status object
    pub fn status(&self) -> Result<JsValue, JsValue> {
        let status_dto = match self.inner.status {
            GameStatus::Playing => StatusDto {
                status: "Playing".to_string(),
                winner: None,
                reason: None,
            },
            GameStatus::Checkmate(winner) => StatusDto {
                status: "Checkmate".to_string(),
                winner: Some(match winner {
                    Color::Red => "red".to_string(),
                    Color::Black => "black".to_string(),
                }),
                reason: Some("Chiếu bí (Checkmate)".to_string()),
            },
            GameStatus::Stalemate => StatusDto {
                status: "Stalemate".to_string(),
                winner: None,
                reason: Some("Hết nước đi (Stalemate)".to_string()),
            },
        };

        serde_wasm_bindgen::to_value(&status_dto)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Return all 90 board squares
    pub fn get_board(&self) -> Result<JsValue, JsValue> {
        let mut squares = Vec::with_capacity(90);
        for row in 0..10 {
            for col in 0..9 {
                let piece = BoardCoordinate::new(row, col)
                    .and_then(|coord| self.inner.board.get_piece(coord))
                    .map(PieceDto::from_piece);

                squares.push(SquareDto { row, col, piece });
            }
        }

        serde_wasm_bindgen::to_value(&squares)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Return legal destination coordinates `[[to_row, to_col], ...]` for a piece,
    /// strictly excluding illegal moves and three-fold repetition traps.
    pub fn get_legal_moves(&self, row: usize, col: usize) -> Result<JsValue, JsValue> {
        let Some(coord) = BoardCoordinate::new(row, col) else {
            return serde_wasm_bindgen::to_value(&Vec::<[usize; 2]>::new())
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")));
        };

        let legal_moves = self.inner.get_strictly_legal_moves();
        let valid_dests: Vec<[usize; 2]> = legal_moves
            .into_iter()
            .filter(|m| {
                usize::from(m.from_row) == coord.row && usize::from(m.from_col) == coord.col
            })
            .map(|m| [usize::from(m.to_row), usize::from(m.to_col)])
            .collect();

        serde_wasm_bindgen::to_value(&valid_dests)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Check if a move is strictly legal, including three-fold repetition check.
    pub fn is_move_legal(
        &self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
    ) -> bool {
        let (Some(from), Some(to)) = (
            BoardCoordinate::new(from_row, from_col),
            BoardCoordinate::new(to_row, to_col),
        ) else {
            return false;
        };
        self.inner.is_move_legal(from, to)
    }

    /// Return all strictly legal moves as flat array `[from_row, from_col, to_row, to_col, ...]`.
    pub fn get_strictly_legal_moves_flat(&self) -> Result<JsValue, JsValue> {
        let moves = self.inner.get_strictly_legal_moves();
        let mut flat = Vec::with_capacity(moves.len() * 4);
        for m in moves {
            flat.push(usize::from(m.from_row));
            flat.push(usize::from(m.from_col));
            flat.push(usize::from(m.to_row));
            flat.push(usize::from(m.to_col));
        }
        serde_wasm_bindgen::to_value(&flat)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Execute a move from (from_row, from_col) to (to_row, to_col)
    pub fn make_move(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
    ) -> Result<JsValue, JsValue> {
        let from = BoardCoordinate::new(from_row, from_col)
            .ok_or_else(|| JsValue::from_str("Invalid from coordinate"))?;
        let to = BoardCoordinate::new(to_row, to_col)
            .ok_or_else(|| JsValue::from_str("Invalid to coordinate"))?;

        let captured = self.inner.board.get_piece(to);

        self.inner
            .make_move(from, to)
            .map_err(|e| JsValue::from_str(&format!("Illegal move: {e:?}")))?;

        let in_check = is_in_check(&self.inner.board, self.inner.turn);
        let result = MoveResultDto {
            success: true,
            captured: captured.map(PieceDto::from_piece),
            in_check,
        };

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Undo the last move
    pub fn undo_move(&mut self) -> bool {
        self.inner.undo_move()
    }

    /// Get history records
    pub fn get_history(&self) -> Result<JsValue, JsValue> {
        let records: Vec<HistoryRecordDto> = self
            .inner
            .history
            .iter()
            .map(HistoryRecordDto::from_record)
            .collect();

        serde_wasm_bindgen::to_value(&records)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Add a note to the last move (e.g. AI stats)
    pub fn annotate_last_move(&mut self, note: &str) {
        if let Some(last) = self.inner.history.last_mut() {
            last.note = Some(note.to_string());
        }
    }

    /// Look up an opening book move if available
    pub fn get_book_move(&self) -> Result<JsValue, JsValue> {
        let book_move = opening::get_book_move(&self.inner.board, self.inner.turn);
        let result = book_move.map(|(from, to)| MoveDto {
            from_row: from.row,
            from_col: from.col,
            to_row: to.row,
            to_col: to.col,
        });

        serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Number of moves played so far
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.inner.history.len()
    }

    /// Export entire game state to JSON string
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }

    /// Load game state from JSON string, preserving full move history
    pub fn from_json(json_str: &str) -> Result<WasmGameState, JsValue> {
        let mut state: GameState = serde_json::from_str(json_str)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {e}")))?;
        state.board.refresh_nnue();
        Ok(Self { inner: state })
    }
}

impl Default for WasmGameState {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM-compatible wrapper around AlphaBetaEngine
#[wasm_bindgen]
pub struct WasmEngine {
    engine: AlphaBetaEngine,
}

#[wasm_bindgen]
impl WasmEngine {
    /// Initialize AI search engine with optional JSON config
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: Option<String>) -> Result<WasmEngine, JsValue> {
        let config = if let Some(json) = config_json {
            serde_json::from_str::<EngineConfig>(&json)
                .map_err(|e| JsValue::from_str(&format!("Invalid EngineConfig JSON: {e}")))?
        } else {
            EngineConfig::default()
        };

        Ok(Self {
            engine: AlphaBetaEngine::new(Arc::new(config)),
        })
    }

    /// Update engine configuration from JSON
    pub fn update_config(&mut self, config_json: &str) -> Result<(), JsValue> {
        let config = serde_json::from_str::<EngineConfig>(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid EngineConfig JSON: {e}")))?;
        self.engine.update_config(Arc::new(config));
        Ok(())
    }

    /// Clear the transposition table and reset node memory
    pub fn clear_tt(&mut self) {
        self.engine.clear_tt();
    }

    /// Loads NNUE weights dynamically from a Uint8Array / binary slice
    pub fn load_weights_bytes(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        let weights = crate::engine::nnue::NnueWeights::load_from_bytes(bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to load NNUE weights: {e}")))?;
        self.engine.set_eval_weights(Some(Arc::new(weights)));
        Ok(())
    }

    /// Checks if active NNUE weights are loaded and functional
    #[must_use]
    pub fn is_nnue_active(&self) -> bool {
        self.engine.has_eval_weights()
    }

    /// Run AI search on the given game state
    pub fn search(
        &mut self,
        game_state: &WasmGameState,
        time_limit_ms: u64,
        max_depth: Option<u8>,
    ) -> Result<JsValue, JsValue> {
        let limit = match (max_depth, time_limit_ms) {
            (Some(depth), t) if t > 0 => SearchLimit::DepthOrTime { depth, time_ms: t },
            (Some(depth), _) => SearchLimit::Depth(depth),
            (None, t) => SearchLimit::Time(t),
        };

        let result = self.engine.search(&game_state.inner, limit, &[]);

        let res_dto = result.map(|(mv, stats)| SearchResultDto {
            r#move: MoveDto {
                from_row: mv.from_row as usize,
                from_col: mv.from_col as usize,
                to_row: mv.to_row as usize,
                to_col: mv.to_col as usize,
            },
            stats: SearchStatsDto {
                depth: stats.depth,
                nodes: stats.nodes,
                time_ms: stats.time_ms,
            },
        });

        serde_wasm_bindgen::to_value(&res_dto)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
    }
}
