import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Trophy, Handshake, AlertTriangle, RotateCcw } from 'lucide-react';
import init, { WasmGameState } from './wasm/cotuong_core.js';
import wasmUrl from './wasm/cotuong_core_bg.wasm?url';
import type {
  Difficulty,
  EngineConfig,
  GameMode,
  GameStatus,
  HistoryRecord,
  MoveCoords,
  PieceColor,
  SearchStats,
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
  const [lastStats, setLastStats] = useState<SearchStats | null>(null);

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

  // UI Panels State
  const [isLogOpen, setIsLogOpen] = useState(false);
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

          setLastStats(stats);
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
          // Safety recovery: if AI chosen move failed, attempt to play the first strictly legal move
          try {
            const fallbackMoves = state.get_strictly_legal_moves_flat() as number[];
            if (fallbackMoves.length >= 4) {
              const fbMove: MoveCoords = {
                from_row: fallbackMoves[0]!,
                from_col: fallbackMoves[1]!,
                to_row: fallbackMoves[2]!,
                to_col: fallbackMoves[3]!,
              };
              const fbRes = state.make_move(
                fbMove.from_row,
                fbMove.from_col,
                fbMove.to_row,
                fbMove.to_col
              ) as { success: boolean; captured: unknown; in_check: boolean };
              state.annotate_last_move('🤖 Nước đi hợp lệ thay thế');
              setLastMove(fbMove);
              syncGameState();
              if (fbRes.in_check) {
                soundManager.play('check');
              } else if (fbRes.captured) {
                soundManager.play('capture');
              } else {
                soundManager.play('move');
              }
            }
          } catch (recoveryErr) {
            console.error('Error applying recovery AI move:', recoveryErr);
          }
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
      stateJson: state.to_json(),
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

    if (gameMode === 'HumanVsComputer' && turn !== playerSide) return;
    if (gameMode === 'ComputerVsComputer') return;

    const clickedSquare = board.find((s) => s.row === row && s.col === col);
    const piece = clickedSquare?.piece;

    if (selectedSquare) {
      const [selRow, selCol] = selectedSquare;

      if (selRow === row && selCol === col) {
        setSelectedSquare(null);
        setLegalMoves([]);
        return;
      }

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
    setLastStats(null);
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

  return (
    <div className="min-h-screen bg-[#14110F] text-[#F5EFE6] flex flex-col justify-between py-4 sm:py-5">
      {/* Top Bar */}
      <header className="w-full max-w-[760px] mx-auto px-3 sm:px-6 flex items-center justify-between gap-4 pb-3 mb-3 border-b border-stone-800/80">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-full bg-gradient-to-br from-red-700 to-red-950 border border-amber-400/50 flex items-center justify-center font-display-serif font-bold text-lg text-amber-100 shadow-md shrink-0">
            帥
          </div>
          <div>
            <h1 className="font-display-serif text-lg sm:text-xl font-bold tracking-wide text-[#F5EFE6] leading-tight">
              CỜ TƯỚNG ĐẠI SƯ
            </h1>
            <p className="text-xs text-stone-400">
              Xiangqi AI · WebAssembly Engine
            </p>
          </div>
        </div>

        {/* Active Turn & Check Status Indicator */}
        <div className="flex items-center gap-3">
          {isInCheck && status.status === 'Playing' && (
            <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold">
              <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
              <span>Chiếu tướng!</span>
            </div>
          )}

          <div className="flex items-center gap-2 px-3.5 py-1.5 rounded-xl bg-[#1E1915] border border-stone-800">
            <span
              className={`w-5 h-5 rounded-full flex items-center justify-center font-display-serif text-xs font-bold ${
                turn === 'red'
                  ? 'bg-red-900 text-red-200 border border-red-500/50'
                  : 'bg-stone-800 text-stone-200 border border-stone-600'
              }`}
            >
              {turn === 'red' ? '帥' : '將'}
            </span>
            <div className="text-xs">
              <span className="text-stone-400">Lượt đi: </span>
              <span
                className={
                  turn === 'red'
                    ? 'font-semibold text-red-400'
                    : 'font-semibold text-amber-300'
                }
              >
                {turn === 'red' ? 'Bên Đỏ' : 'Bên Đen'}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col items-center justify-center gap-4 w-full mx-auto">
        <div className="w-full px-3 sm:px-6">
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
            onToggleLog={() => setIsLogOpen((prev) => !prev)}
            moveCount={history.length}
            isThinking={isThinking}
            canUndo={history.length > 0}
          />
        </div>

        {/* Board Container with Overview Overlays */}
        <div className="relative flex flex-col items-center justify-center my-1">
          <BoardCanvas
            board={board}
            playerSide={playerSide}
            selectedSquare={selectedSquare}
            legalMoves={legalMoves}
            lastMove={lastMove}
            onSquareClick={handleSquareClick}
            disabled={isThinking || status.status !== 'Playing'}
          >
            {/* AI Thinking Overview Overlay directly on the board */}
            <ThinkingIndicator
              isThinking={isThinking}
              turn={turn}
              difficulty={difficulty}
              lastStats={lastStats}
            />

            {/* End-of-Game Victory / Draw Overlay on the board */}
            {status.status !== 'Playing' && (
              <div className="absolute inset-0 z-30 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs rounded-2xl animate-board-overlay">
                <div className="w-full max-w-[320px] rounded-2xl bg-[#1A1512] border border-amber-500/40 shadow-2xl p-5 text-center">
                  {status.status === 'Checkmate' ? (
                    <>
                      <div className="w-12 h-12 rounded-full bg-amber-500/15 border border-amber-500/35 flex items-center justify-center mx-auto mb-3 text-amber-400">
                        <Trophy className="w-6 h-6" />
                      </div>
                      <h2 className="font-display-serif text-lg font-bold text-[#F5EFE6]">
                        {status.winner === 'red' ? 'Bên Đỏ Chiến Thắng!' : 'Bên Đen Chiến Thắng!'}
                      </h2>
                      <p className="text-xs text-stone-400 mt-1">
                        Chiếu bí (Checkmate) sau {history.length} nước đi
                      </p>
                    </>
                  ) : (
                    <>
                      <div className="w-12 h-12 rounded-full bg-stone-800 border border-stone-700 flex items-center justify-center mx-auto mb-3 text-stone-300">
                        <Handshake className="w-6 h-6" />
                      </div>
                      <h2 className="font-display-serif text-lg font-bold text-[#F5EFE6]">
                        Ván Cờ Hòa!
                      </h2>
                      <p className="text-xs text-stone-400 mt-1">
                        Hết nước đi hợp lệ (Stalemate)
                      </p>
                    </>
                  )}

                  <button
                    type="button"
                    onClick={handleNewGame}
                    className="mt-4 w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] text-white transition-colors cursor-pointer shadow-md"
                  >
                    <RotateCcw className="w-4 h-4" />
                    <span>Chơi ván mới</span>
                  </button>
                </div>
              </div>
            )}
          </BoardCanvas>
        </div>

        {/* Collapsible Move Log Dock at the bottom ("ẩn xuống, khi nào cần thì mở lên xem") */}
        <LogPanel
          history={history}
          isOpen={isLogOpen}
          onToggle={setIsLogOpen}
          onExportCsv={() => exportCsv(history)}
        />
      </main>

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
