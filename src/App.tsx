import React, { useState, useEffect, useRef, useCallback } from 'react';
import init, { WasmGameState } from './wasm/cotuong_core.js';
import wasmUrl from './wasm/cotuong_core_bg.wasm?url';
import type {
  Difficulty,
  EngineConfig,
  GameMode,
  GameStatus,
  HistoryRecord,
  PieceColor,
  Square,
} from './types/game.js';
import { DEFAULT_ENGINE_CONFIG } from './types/game.js';
import { BoardCanvas } from './components/BoardCanvas.js';
import { ControlsArea } from './components/ControlsArea.js';
import { LogPanel } from './components/LogPanel.js';
import { ThinkingIndicator } from './components/ThinkingIndicator.js';
import { ConfigModal } from './components/ConfigModal.js';
import { soundManager } from './utils/sound.js';
import { exportCsv } from './utils/export.js';
import './App.css';

const DIFFICULTY_TIMES: Record<Difficulty, number> = {
  1: 1000,
  2: 2000,
  3: 5000,
  4: 10000,
  5: 20000,
};

export const App: React.FC = () => {
  const [isWasmReady, setIsWasmReady] = useState(false);
  const gameStateRef = useRef<WasmGameState | null>(null);

  // Game Settings & State
  const [gameMode, setGameMode] = useState<GameMode>('HumanVsComputer');
  const [playerSide, setPlayerSide] = useState<PieceColor>('red');
  const [difficulty, setDifficulty] = useState<Difficulty>(1);
  const [isPaused, setIsPaused] = useState(false);
  const [isThinking, setIsThinking] = useState(false);

  // Board presentation
  const [board, setBoard] = useState<Square[]>([]);
  const [turn, setTurn] = useState<PieceColor>('red');
  const [status, setStatus] = useState<GameStatus>({ status: 'Playing' });
  const [isInCheck, setIsInCheck] = useState(false);
  const [selectedSquare, setSelectedSquare] = useState<[number, number] | null>(null);
  const [legalMoves, setLegalMoves] = useState<[number, number][]>([]);
  const [lastMove, setLastMove] = useState<{
    from_row: number;
    from_col: number;
    to_row: number;
    to_col: number;
  } | null>(null);
  const [history, setHistory] = useState<HistoryRecord[]>([]);

  // Engine Dual Configuration
  const [redConfig, setRedConfig] = useState<EngineConfig>(DEFAULT_ENGINE_CONFIG);
  const [blackConfig, setBlackConfig] = useState<EngineConfig>(DEFAULT_ENGINE_CONFIG);
  const [isConfigOpen, setIsConfigOpen] = useState(false);

  // AI Web Worker
  const workerRef = useRef<Worker | null>(null);

  // Sync state from WasmGameState instance
  const syncGameState = useCallback(() => {
    const state = gameStateRef.current;
    if (!state) return;

    setBoard(state.get_board() as Square[]);
    setTurn(state.turn() as PieceColor);
    setStatus(state.status() as GameStatus);
    setIsInCheck(state.is_in_check());
    setHistory(state.get_history() as HistoryRecord[]);
  }, []);

  // Initialize WASM and Worker
  useEffect(() => {
    let worker: Worker | null = null;

    async function setup() {
      await init(wasmUrl);
      gameStateRef.current = new WasmGameState();
      syncGameState();
      setIsWasmReady(true);

      worker = new Worker(new URL('./workers/ai.worker.ts', import.meta.url), {
        type: 'module',
      });
      workerRef.current = worker;
    }

    setup().catch((err) => {
      console.error('Failed to initialize WASM or worker:', err);
    });

    return () => {
      worker?.terminate();
    };
  }, [syncGameState]);

  // AI Search Trigger
  const triggerAiMove = useCallback(() => {
    const state = gameStateRef.current;
    const worker = workerRef.current;
    if (!state || !worker) return;

    const currentStatus = state.status() as GameStatus;
    if (currentStatus.status !== 'Playing') return;

    setIsThinking(true);

    // 1. Check opening book move first
    const bookMove = state.get_book_move() as {
      from_row: number;
      from_col: number;
      to_row: number;
      to_col: number;
    } | null;

    if (bookMove) {
      setTimeout(() => {
        try {
          const res = state.make_move(
            bookMove.from_row,
            bookMove.from_col,
            bookMove.to_row,
            bookMove.to_col
          ) as { success: boolean; captured: unknown; in_check: boolean };

          state.annotate_last_move('📖 Khai cuộc (Book Move)');
          setLastMove(bookMove);
          syncGameState();

          const newStatus = state.status() as GameStatus;
          if (newStatus.status === 'Checkmate') {
            soundManager.play('checkmate');
          } else if (res.in_check) {
            soundManager.play('check');
          } else if (res.captured) {
            soundManager.play('capture');
          } else {
            soundManager.play('move');
          }
        } catch (e) {
          console.error('Failed to apply book move:', e);
        } finally {
          setIsThinking(false);
        }
      }, 250);
      return;
    }

    // 2. Offload to Web Worker
    const currentTurn = state.turn() as PieceColor;
    const config = currentTurn === 'red' ? redConfig : blackConfig;
    const timeLimitMs = DIFFICULTY_TIMES[difficulty];

    const handleWorkerMessage = (e: MessageEvent) => {
      if (e.data.type === 'SEARCH_SUCCESS') {
        const { move, stats } = e.data.result;
        try {
          const res = state.make_move(
            move.from_row,
            move.from_col,
            move.to_row,
            move.to_col
          ) as { success: boolean; captured: unknown; in_check: boolean };

          state.annotate_last_move(
            `🤖 Độ sâu: ${stats.depth}, Nodes: ${stats.nodes}, Thời gian: ${stats.time_ms}ms`
          );

          setLastMove(move);
          syncGameState();

          const newStatus = state.status() as GameStatus;
          if (newStatus.status === 'Checkmate') {
            soundManager.play('checkmate');
          } else if (res.in_check) {
            soundManager.play('check');
          } else if (res.captured) {
            soundManager.play('capture');
          } else {
            soundManager.play('move');
          }
        } catch (err) {
          console.error('Error applying AI move:', err);
        } finally {
          setIsThinking(false);
          worker.removeEventListener('message', handleWorkerMessage);
        }
      } else if (e.data.type === 'NO_MOVE' || e.data.type === 'SEARCH_ERROR') {
        setIsThinking(false);
        worker.removeEventListener('message', handleWorkerMessage);
      }
    };

    worker.addEventListener('message', handleWorkerMessage);
    worker.postMessage({
      type: 'SEARCH',
      fen: state.to_fen(),
      timeLimitMs,
      configJson: JSON.stringify(config),
    });
  }, [difficulty, redConfig, blackConfig, syncGameState]);

  // Handle Game Loop (Check whether AI should play)
  useEffect(() => {
    if (!isWasmReady || isThinking) return;
    const state = gameStateRef.current;
    if (!state) return;

    const currentStatus = state.status() as GameStatus;
    if (currentStatus.status !== 'Playing') return;

    const currentTurn = state.turn() as PieceColor;

    if (gameMode === 'HumanVsComputer') {
      if (currentTurn !== playerSide) {
        triggerAiMove();
      }
    } else if (gameMode === 'ComputerVsComputer') {
      if (!isPaused) {
        const timer = setTimeout(() => {
          triggerAiMove();
        }, 300);
        return () => clearTimeout(timer);
      }
    }
  }, [
    isWasmReady,
    turn,
    gameMode,
    playerSide,
    isPaused,
    isThinking,
    triggerAiMove,
  ]);

  // Handle Board Click
  const handleSquareClick = (row: number, col: number) => {
    if (isThinking || status.status !== 'Playing') return;
    const state = gameStateRef.current;
    if (!state) return;

    // Check if player is allowed to move right now
    if (gameMode === 'HumanVsComputer' && turn !== playerSide) return;
    if (gameMode === 'ComputerVsComputer') return;

    const clickedSquare = board.find((s) => s.row === row && s.col === col);
    const piece = clickedSquare?.piece;

    // If already selected a piece
    if (selectedSquare) {
      const [selRow, selCol] = selectedSquare;

      // Clicked same square -> deselect
      if (selRow === row && selCol === col) {
        setSelectedSquare(null);
        setLegalMoves([]);
        return;
      }

      // Check if clicked square is in legal moves
      const isLegal = legalMoves.some(([r, c]) => r === row && c === col);
      if (isLegal) {
        try {
          const moveRes = state.make_move(selRow, selCol, row, col) as {
            success: boolean;
            captured: unknown;
            in_check: boolean;
          };

          setLastMove({ from_row: selRow, from_col: selCol, to_row: row, to_col: col });
          setSelectedSquare(null);
          setLegalMoves([]);
          syncGameState();

          const newStatus = state.status() as GameStatus;
          if (newStatus.status === 'Checkmate') {
            soundManager.play('checkmate');
          } else if (moveRes.in_check) {
            soundManager.play('check');
          } else if (moveRes.captured) {
            soundManager.play('capture');
          } else {
            soundManager.play('move');
          }
        } catch (e) {
          console.error('Move error:', e);
        }
        return;
      }
    }

    // Select own piece
    if (piece && piece.color === turn) {
      setSelectedSquare([row, col]);
      const moves = state.get_legal_moves(row, col) as [number, number][];
      setLegalMoves(moves);
    } else {
      setSelectedSquare(null);
      setLegalMoves([]);
    }
  };

  // New Game
  const handleNewGame = () => {
    if (isThinking) return;
    gameStateRef.current = new WasmGameState();
    setSelectedSquare(null);
    setLegalMoves([]);
    setLastMove(null);
    syncGameState();
    if (gameMode === 'ComputerVsComputer') {
      setIsPaused(true);
    }
  };

  // Undo Move
  const handleUndo = () => {
    if (isThinking) return;
    const state = gameStateRef.current;
    if (!state) return;

    if (gameMode === 'HumanVsComputer') {
      // Undo both AI and player move if player's turn
      if (state.turn() === playerSide && state.history_len() >= 2) {
        state.undo_move();
      }
      state.undo_move();
    } else {
      state.undo_move();
    }

    setSelectedSquare(null);
    setLegalMoves([]);
    setLastMove(null);
    syncGameState();
  };

  const getStatusBanner = () => {
    if (status.status === 'Checkmate') {
      const winnerName = status.winner === 'red' ? 'Bên ĐỎ' : 'Bên ĐEN';
      return (
        <div className="status-banner status-checkmate">
          🏆 {winnerName} THẮNG! (Chiếu bí - Checkmate)
        </div>
      );
    }
    if (status.status === 'Stalemate') {
      return (
        <div className="status-banner status-stalemate">
          🤝 HÒA CỜ! (Hết nước đi - Stalemate)
        </div>
      );
    }
    if (isInCheck) {
      return (
        <div className="status-banner status-check">
          ⚠️ CHIẾU TƯỚNG!
        </div>
      );
    }
    return null;
  };

  return (
    <div className="app-container">
      <h1 className="game-title">CỜ TƯỚNG</h1>

      {getStatusBanner()}

      <ThinkingIndicator isThinking={isThinking} />

      <ControlsArea
        gameMode={gameMode}
        setGameMode={setGameMode}
        playerSide={playerSide}
        setPlayerSide={setPlayerSide}
        difficulty={difficulty}
        setDifficulty={setDifficulty}
        isPaused={isPaused}
        setIsPaused={setIsPaused}
        onNewGame={handleNewGame}
        onUndo={handleUndo}
        onExportCsv={() => exportCsv(history)}
        onOpenConfig={() => setIsConfigOpen(true)}
        isThinking={isThinking}
        canUndo={history.length > 0}
      />

      <div className="main-layout">
        <LogPanel history={history} />

        <div className="board-container">
          <div className="turn-badge">
            Lượt đi: {turn === 'red' ? '🔴 Bên Đỏ' : '⚫ Bên Đen'}
          </div>

          <BoardCanvas
            board={board}
            playerSide={playerSide}
            selectedSquare={selectedSquare}
            legalMoves={legalMoves}
            lastMove={lastMove}
            onSquareClick={handleSquareClick}
            disabled={isThinking || status.status !== 'Playing'}
          />
        </div>
      </div>

      <ConfigModal
        isOpen={isConfigOpen}
        onClose={() => setIsConfigOpen(false)}
        redConfig={redConfig}
        setRedConfig={setRedConfig}
        blackConfig={blackConfig}
        setBlackConfig={setBlackConfig}
      />
    </div>
  );
};

export default App;

