use cotuong_core::engine::Move;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EngineProtocol {
    #[default]
    Uci,
    Ucci,
}

pub struct ExternalEngineClient {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    protocol: EngineProtocol,
}

impl ExternalEngineClient {
    /// Launches an external engine process and performs the initial protocol handshake.
    pub fn spawn(engine_path: &str, protocol: EngineProtocol) -> Result<Self, String> {
        let raw_path = std::path::Path::new(engine_path);
        let abs_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else if let Ok(canon) = std::fs::canonicalize(raw_path) {
            canon
        } else if let Ok(cwd) = std::env::current_dir() {
            cwd.join(raw_path)
        } else {
            raw_path.to_path_buf()
        };

        if !abs_path.exists() {
            return Err(format!(
                "Engine executable not found at '{}' (resolved to '{}')",
                engine_path,
                abs_path.display()
            ));
        }

        let mut cmd = Command::new(&abs_path);
        if let Some(parent) = abs_path.parent() {
            if !parent.as_os_str().is_empty() {
                cmd.current_dir(parent);
            }
        }

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to start engine process '{engine_path}' (at '{}'): {e}",
                    abs_path.display()
                )
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture stdin of engine process".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout of engine process".to_string())?;

        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line_res in reader.lines() {
                match line_res {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let mut client = Self {
            child,
            stdin,
            rx,
            protocol,
        };

        client.handshake()?;
        Ok(client)
    }

    /// Performs the handshake according to protocol (UCCI or UCI) with non-blocking fallback.
    fn handshake(&mut self) -> Result<(), String> {
        match self.protocol {
            EngineProtocol::Ucci => {
                self.send_line("ucci")?;
                match self.wait_for_token("ucciok", Duration::from_secs(2)) {
                    Ok(_) => {
                        let _ = self.send_line("setoption name Threads value 1");
                        let _ = self.send_line("setoption name Hash value 32");
                        self.send_line("isready")?;
                        self.wait_for_token("readyok", Duration::from_secs(5))?;
                    }
                    Err(_) => {
                        // Fallback automatically to UCI protocol if engine does not support UCCI
                        self.protocol = EngineProtocol::Uci;
                        self.send_line("uci")?;
                        self.wait_for_token("uciok", Duration::from_secs(5))?;
                        let _ = self.send_line("setoption name UCI_Variant value xiangqi");
                        let _ = self.send_line("setoption name Threads value 1");
                        let _ = self.send_line("setoption name Hash value 32");
                        self.send_line("isready")?;
                        self.wait_for_token("readyok", Duration::from_secs(5))?;
                    }
                }
            }
            EngineProtocol::Uci => {
                self.send_line("uci")?;
                self.wait_for_token("uciok", Duration::from_secs(5))?;
                // Configure variant for Fairy-Stockfish if needed
                let _ = self.send_line("setoption name UCI_Variant value xiangqi");
                let _ = self.send_line("setoption name Threads value 1");
                let _ = self.send_line("setoption name Hash value 32");
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

    /// Reads output lines non-blockingly until finding a token or timeout.
    fn wait_for_token(&mut self, token: &str, timeout: Duration) -> Result<String, String> {
        let start = Instant::now();

        loop {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                return Err(format!("Timeout waiting for engine token: '{token}'"));
            }
            let remaining = timeout - elapsed;

            match self.rx.recv_timeout(remaining) {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.contains(token) || trimmed.starts_with(token) {
                        return Ok(trimmed.to_string());
                    }
                    if trimmed.contains("Unknown command") {
                        return Err(format!("Unknown command error: {trimmed}"));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(format!("Timeout waiting for engine token: '{token}'"));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("Engine process closed its stdout unexpectedly".to_string());
                }
            }
        }
    }

    /// Notifies the engine of a new game and resets transposition tables / buffer queues.
    pub fn new_game(&mut self) -> Result<(), String> {
        while self.rx.try_recv().is_ok() {}

        match self.protocol {
            EngineProtocol::Uci => {
                let _ = self.send_line("ucinewgame");
                let _ = self.send_line("isready");
                let _ = self.wait_for_token("readyok", Duration::from_secs(3));
            }
            EngineProtocol::Ucci => {
                let _ = self.send_line("newgame");
                let _ = self.send_line("isready");
                let _ = self.wait_for_token("readyok", Duration::from_secs(3));
            }
        }

        while self.rx.try_recv().is_ok() {}
        Ok(())
    }

    /// Evaluates a board position specified by FEN up to the given search depth.
    /// Returns (centipawn_score_from_side_to_move_perspective, Option<best_move>).
    pub fn evaluate_fen(
        &mut self,
        fen: &str,
        depth: u8,
        timeout: Duration,
    ) -> Result<(f32, Option<Move>), String> {
        // Drain any stale messages left in the channel before sending the new command
        while self.rx.try_recv().is_ok() {}

        self.send_line(&format!("position fen {fen}"))?;
        self.send_line(&format!("go depth {depth}"))?;

        let start = Instant::now();
        let mut last_eval_cp = 0.0f32;
        let mut best_move: Option<Move> = None;

        loop {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                let _ = self.send_line("stop");
                let stop_start = Instant::now();
                while stop_start.elapsed() < Duration::from_millis(1000) {
                    if let Ok(line) = self.rx.recv_timeout(Duration::from_millis(100)) {
                        let trimmed = line.trim();
                        if trimmed.starts_with("bestmove") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 && parts[1] != "nobestmove" && parts[1] != "(none)"
                            {
                                best_move = Move::from_uci(parts[1]).ok();
                            }
                            break;
                        }
                    } else {
                        break;
                    }
                }
                while self.rx.try_recv().is_ok() {}
                break;
            }
            let remaining = timeout - elapsed;

            match self.rx.recv_timeout(remaining) {
                Ok(line) => {
                    let trimmed = line.trim();

                    if trimmed.starts_with("info") {
                        if let Some(mate_idx) = trimmed.find("score mate ") {
                            let sub = &trimmed[mate_idx + 11..];
                            if let Some(val_str) = sub.split_whitespace().next() {
                                if let Ok(val) = val_str.parse::<f32>() {
                                    last_eval_cp = if val > 0.0 { 10000.0 } else { -10000.0 };
                                }
                            }
                        } else if let Some(cp_idx) = trimmed.find("score cp ") {
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
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.send_line("stop");
                    let stop_start = Instant::now();
                    while stop_start.elapsed() < Duration::from_millis(1000) {
                        if let Ok(line) = self.rx.recv_timeout(Duration::from_millis(100)) {
                            let trimmed = line.trim();
                            if trimmed.starts_with("bestmove") {
                                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                if parts.len() >= 2
                                    && parts[1] != "nobestmove"
                                    && parts[1] != "(none)"
                                {
                                    best_move = Move::from_uci(parts[1]).ok();
                                }
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    while self.rx.try_recv().is_ok() {}
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        Ok((last_eval_cp, best_move))
    }

    /// Evaluates a board position reached via move history from startpos.
    /// Providing full move history ensures engines track repetition rules accurately.
    pub fn evaluate_game_position(
        &mut self,
        moves: &[Move],
        depth: u8,
        timeout: Duration,
    ) -> Result<(f32, Option<Move>), String> {
        // Drain any stale messages left in the channel before sending the new command
        while self.rx.try_recv().is_ok() {}

        let cmd = if moves.is_empty() {
            "position startpos".to_string()
        } else {
            let moves_str: String = moves.iter().map(Move::to_uci).collect::<Vec<_>>().join(" ");
            format!("position startpos moves {moves_str}")
        };

        self.send_line(&cmd)?;
        self.send_line(&format!("go depth {depth}"))?;

        let start = Instant::now();
        let mut last_eval_cp = 0.0f32;
        let mut best_move: Option<Move> = None;

        loop {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                let _ = self.send_line("stop");
                let stop_start = Instant::now();
                while stop_start.elapsed() < Duration::from_millis(1000) {
                    if let Ok(line) = self.rx.recv_timeout(Duration::from_millis(100)) {
                        let trimmed = line.trim();
                        if trimmed.starts_with("bestmove") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 && parts[1] != "nobestmove" && parts[1] != "(none)"
                            {
                                best_move = Move::from_uci(parts[1]).ok();
                            }
                            break;
                        }
                    } else {
                        break;
                    }
                }
                while self.rx.try_recv().is_ok() {}
                break;
            }
            let remaining = timeout - elapsed;

            match self.rx.recv_timeout(remaining) {
                Ok(line) => {
                    let trimmed = line.trim();

                    if trimmed.starts_with("info") {
                        if let Some(mate_idx) = trimmed.find("score mate ") {
                            let sub = &trimmed[mate_idx + 11..];
                            if let Some(val_str) = sub.split_whitespace().next() {
                                if let Ok(val) = val_str.parse::<f32>() {
                                    last_eval_cp = if val > 0.0 { 10000.0 } else { -10000.0 };
                                }
                            }
                        } else if let Some(cp_idx) = trimmed.find("score cp ") {
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
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.send_line("stop");
                    let stop_start = Instant::now();
                    while stop_start.elapsed() < Duration::from_millis(1000) {
                        if let Ok(line) = self.rx.recv_timeout(Duration::from_millis(100)) {
                            let trimmed = line.trim();
                            if trimmed.starts_with("bestmove") {
                                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                                if parts.len() >= 2
                                    && parts[1] != "nobestmove"
                                    && parts[1] != "(none)"
                                {
                                    best_move = Move::from_uci(parts[1]).ok();
                                }
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    while self.rx.try_recv().is_ok() {}
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    break;
                }
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

        loop {
            let elapsed = start.elapsed();
            if elapsed > timeout {
                let _ = self.send_line("stop");
                return Ok(None);
            }
            let remaining = timeout - elapsed;

            match self.rx.recv_timeout(remaining) {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("bestmove") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 2 && parts[1] != "nobestmove" && parts[1] != "(none)" {
                            return Ok(Move::from_uci(parts[1]).ok());
                        }
                        return Ok(None);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    let _ = self.send_line("stop");
                    return Ok(None);
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Ok(None);
                }
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
