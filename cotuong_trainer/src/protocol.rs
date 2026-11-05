use cotuong_core::engine::Move;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineProtocol {
    #[default]
    Ucci,
    Uci,
}

pub struct ExternalEngineClient {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    protocol: EngineProtocol,
}

impl ExternalEngineClient {
    /// Launches an external engine process and performs the initial protocol handshake.
    pub fn spawn(engine_path: &str, protocol: EngineProtocol) -> Result<Self, String> {
        let mut child = Command::new(engine_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start engine process '{engine_path}': {e}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture stdin of engine process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout of engine process".to_string())?;
        let reader = BufReader::new(stdout);

        let mut client = Self {
            child,
            stdin,
            reader,
            protocol,
        };

        client.handshake()?;
        Ok(client)
    }

    /// Performs the handshake according to protocol (UCCI or UCI).
    fn handshake(&mut self) -> Result<(), String> {
        match self.protocol {
            EngineProtocol::Ucci => {
                self.send_line("ucci")?;
                self.wait_for_token("ucciok", Duration::from_secs(5))?;
                self.send_line("isready")?;
                self.wait_for_token("readyok", Duration::from_secs(5))?;
            }
            EngineProtocol::Uci => {
                self.send_line("uci")?;
                self.wait_for_token("uciok", Duration::from_secs(5))?;
                // Configure variant for Fairy-Stockfish if needed
                let _ = self.send_line("setoption name UCI_Variant value xiangqi");
                self.send_line("isready")?;
                self.wait_for_token("readyok", Duration::from_secs(5))?;
            }
        }
        Ok(())
    }

    /// Sends a raw command line to the engine.
    pub fn send_line(&mut self, line: &str) -> Result<(), String> {
        writeln!(self.stdin, "{line}")
            .map_err(|e| format!("Failed to send line '{line}' to engine: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Failed to flush engine stdin: {e}"))
    }

    /// Reads output lines until finding a line that contains the specified token or matches prefix.
    fn wait_for_token(&mut self, token: &str, timeout: Duration) -> Result<String, String> {
        let start = Instant::now();
        let mut line = String::new();

        loop {
            if start.elapsed() > timeout {
                return Err(format!("Timeout waiting for engine token: '{token}'"));
            }

            line.clear();
            let bytes_read = self
                .reader
                .read_line(&mut line)
                .map_err(|e| format!("Error reading from engine: {e}"))?;

            if bytes_read == 0 {
                return Err("Engine process closed its stdout unexpectedly".to_string());
            }

            let trimmed = line.trim();
            if trimmed.contains(token) || trimmed.starts_with(token) {
                return Ok(trimmed.to_string());
            }
        }
    }

    /// Evaluates a board position specified by FEN up to the given search depth.
    /// Returns (centipawn_score_from_side_to_move_perspective, Option<best_move>).
    pub fn evaluate_fen(
        &mut self,
        fen: &str,
        depth: u8,
        timeout: Duration,
    ) -> Result<(f32, Option<Move>), String> {
        self.send_line(&format!("position fen {fen}"))?;
        self.send_line(&format!("go depth {depth}"))?;

        let start = Instant::now();
        let mut line = String::new();
        let mut last_eval_cp = 0.0f32;
        let mut best_move: Option<Move> = None;

        loop {
            if start.elapsed() > timeout {
                let _ = self.send_line("stop");
                break;
            }

            line.clear();
            let bytes = self
                .reader
                .read_line(&mut line)
                .map_err(|e| format!("Error reading engine eval output: {e}"))?;

            if bytes == 0 {
                break;
            }

            let trimmed = line.trim();

            // Parse score from info line: e.g. "info depth 8 score cp 45 ..." or "score 45"
            if trimmed.starts_with("info") {
                if let Some(cp_idx) = trimmed.find("score cp ") {
                    let sub = &trimmed[cp_idx + 9..];
                    if let Some(val_str) = sub.split_whitespace().next() {
                        if let Ok(val) = val_str.parse::<f32>() {
                            last_eval_cp = val;
                        }
                    }
                } else if let Some(score_idx) = trimmed.find("score ") {
                    let sub = &trimmed[score_idx + 6..];
                    if let Some(val_str) = sub.split_whitespace().next() {
                        if let Ok(val) = val_str.parse::<f32>() {
                            last_eval_cp = val;
                        }
                    }
                } else if let Some(mate_idx) = trimmed.find("score mate ") {
                    let sub = &trimmed[mate_idx + 11..];
                    if let Some(val_str) = sub.split_whitespace().next() {
                        if let Ok(val) = val_str.parse::<f32>() {
                            last_eval_cp = if val > 0.0 { 10000.0 } else { -10000.0 };
                        }
                    }
                }
            }

            if trimmed.starts_with("bestmove") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] != "nobestmove" && parts[1] != "(none)" {
                    best_move = Move::from_uci(parts[1]).ok();
                }
                break;
            }
        }

        Ok((last_eval_cp, best_move))
    }

    /// Asks the engine for the best move in a position with a sequence of moves from startpos.
    pub fn get_best_move(
        &mut self,
        fen: &str,
        moves: &[Move],
        depth: u8,
        timeout: Duration,
    ) -> Result<Option<Move>, String> {
        let moves_str: String = moves.iter().map(Move::to_uci).collect::<Vec<_>>().join(" ");

        let pos_cmd = if moves_str.is_empty() {
            format!("position fen {fen}")
        } else {
            format!("position fen {fen} moves {moves_str}")
        };

        self.send_line(&pos_cmd)?;
        self.send_line(&format!("go depth {depth}"))?;

        let start = Instant::now();
        let mut line = String::new();

        loop {
            if start.elapsed() > timeout {
                let _ = self.send_line("stop");
                return Ok(None);
            }

            line.clear();
            let bytes = self
                .reader
                .read_line(&mut line)
                .map_err(|e| format!("Error reading bestmove from engine: {e}"))?;

            if bytes == 0 {
                return Ok(None);
            }

            let trimmed = line.trim();
            if trimmed.starts_with("bestmove") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] != "nobestmove" && parts[1] != "(none)" {
                    return Ok(Move::from_uci(parts[1]).ok());
                }
                return Ok(None);
            }
        }
    }
}

impl Drop for ExternalEngineClient {
    fn drop(&mut self) {
        let _ = self.send_line("quit");
        let _ = self.child.kill();
    }
}
