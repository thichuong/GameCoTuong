import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { Trophy, Handshake, AlertTriangle, RotateCcw, LayoutGrid, Columns, Rows } from 'lucide-react';
import init, { WasmGameState } from './wasm/cotuong_core.js';
import wasmUrl from './wasm/cotuong_core_bg.wasm?url';
import type {
  Difficulty,
  EngineConfig,
  GameMode,
  GameStatus,
  HistoryRecord,
  MoveCoords,
  Piece,
  PieceColor,
  PuzzleStage,
  PuzzleSubMode,
  SearchStats,
  Square,
} from './types/game.js';
import { DEFAULT_ENGINE_CONFIG } from './types/game.js';
import { BoardCanvas } from './components/BoardCanvas.js';
import { ControlsArea } from './components/ControlsArea.js';
import { PuzzleSetupArea } from './components/PuzzleSetupArea.js';
import { LogPanel } from './components/LogPanel.js';
import { ThinkingIndicator } from './components/ThinkingIndicator.js';
import { ConfigModal } from './components/ConfigModal.js';
import { CapturedPiecesTray } from './components/CapturedPiecesTray.js';
import {
  extractCapturedPieces,
  determineLayoutOrientation,
  calculateOptimalBoardSize,
  type LayoutOrientation,
} from './utils/capturedPieces.js';
import { soundManager } from './utils/sound.js';
import { exportCsv } from './utils/export.js';
import { boardToFen, fenToBoard, CLASSIC_PUZZLES } from './utils/fen.js';
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

  // Puzzle Mode State
  const [puzzleStage, setPuzzleStage] = useState<PuzzleStage>('setup');
  const [puzzleSubMode, setPuzzleSubMode] = useState<PuzzleSubMode>('HumanVsComputer');
  const [puzzleFirstTurn, setPuzzleFirstTurn] = useState<PieceColor>('red');
  const [puzzleSelectedTool, setPuzzleSelectedTool] = useState<Piece | 'erase' | null>(null);
  const [initialPuzzleFen, setInitialPuzzleFen] = useState<string | null>(null);

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

  const redConfigRef = useRef(redConfig);
  redConfigRef.current = redConfig;
  const blackConfigRef = useRef(blackConfig);
  blackConfigRef.current = blackConfig;
  const difficultyRef = useRef(difficulty);
  difficultyRef.current = difficulty;
  const isAiProcessingRef = useRef(false);

  // Responsive Layout Mode & Window Dimensions
  const [layoutOverride, setLayoutOverride] = useState<'auto' | 'sides' | 'stacked'>('auto');
  const [windowDims, setWindowDims] = useState(() => ({
    width: typeof window !== 'undefined' ? window.innerWidth : 1200,
    height: typeof window !== 'undefined' ? window.innerHeight : 800,
  }));

  useEffect(() => {
    const handleResize = () => {
      setWindowDims({
        width: window.innerWidth,
        height: window.innerHeight,
      });
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Compute layout orientation based on aspect ratio or user override
  const autoOrientation = useMemo(
    () => determineLayoutOrientation(windowDims.width, windowDims.height),
    [windowDims.width, windowDims.height]
  );
  const layoutOrientation: LayoutOrientation =
    layoutOverride === 'auto' ? autoOrientation : layoutOverride;

  const boardSize = useMemo(
    () => calculateOptimalBoardSize(windowDims.width, windowDims.height, layoutOrientation),
    [windowDims.width, windowDims.height, layoutOrientation]
  );

  // Extract captured pieces summary
  const capturedSummary = useMemo(
    () => extractCapturedPieces(history),
    [history]
  );

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
    if (isAiProcessingRef.current) return;
    const state = gameStateRef.current;
    const worker = workerRef.current;
    if (!state || !worker) return;

    const currentStatus = state.status() as GameStatus;
    if (currentStatus.status !== 'Playing') return;

    isAiProcessingRef.current = true;
    setIsThinking(true);

    // 1. Check opening book move first (only in standard games and strictly verify legality)
    const isPuzzle = gameMode === 'Puzzle';
    const rawBookMove = isPuzzle ? null : (state.get_book_move() as MoveCoords | null);
    const bookMove =
      rawBookMove &&
      state.is_move_legal(
        rawBookMove.from_row,
        rawBookMove.from_col,
        rawBookMove.to_row,
        rawBookMove.to_col
      )
        ? rawBookMove
        : null;

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
          isAiProcessingRef.current = false;
          setIsThinking(false);
        }
      }, 250);
      return;
    }

    // 2. Offload to Web Worker
    const currentTurn = state.turn() as PieceColor;
    const config = currentTurn === 'red' ? redConfigRef.current : blackConfigRef.current;
    const timeLimitMs = DIFFICULTY_TIMES[difficultyRef.current];

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
          isAiProcessingRef.current = false;
          setIsThinking(false);
          worker.removeEventListener('message', handleWorkerMessage);
        }
      } else if (e.data.type === 'NO_MOVE' || e.data.type === 'SEARCH_ERROR') {
        isAiProcessingRef.current = false;
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
  }, [syncGameState, gameMode]);

  // Handle Game Loop (Check whether AI should play)
  useEffect(() => {
    if (!isWasmReady || isThinking) return;
    const state = gameStateRef.current;
    if (!state) return;

    if (gameMode === 'Puzzle' && puzzleStage === 'setup') return;

    const currentStatus = state.status() as GameStatus;
    if (currentStatus.status !== 'Playing') return;

    const currentTurn = state.turn() as PieceColor;
    const effectivePlayMode =
      gameMode === 'Puzzle' ? (puzzleStage === 'playing' ? puzzleSubMode : null) : gameMode;

    if (effectivePlayMode === 'HumanVsComputer') {
      if (currentTurn !== playerSide) {
        triggerAiMove();
      }
    } else if (effectivePlayMode === 'ComputerVsComputer') {
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
    puzzleStage,
    puzzleSubMode,
    triggerAiMove,
  ]);

  // Handle Board Click in Setup Mode
  const handleSetupSquareClick = useCallback(
    (row: number, col: number) => {
      if (puzzleSelectedTool === 'erase') {
        setBoard((prev) =>
          prev.map((sq) => (sq.row === row && sq.col === col ? { ...sq, piece: null } : sq))
        );
        return;
      }

      if (puzzleSelectedTool) {
        setBoard((prev) =>
          prev.map((sq) =>
            sq.row === row && sq.col === col
              ? {
                  ...sq,
                  piece: { type: puzzleSelectedTool.type, color: puzzleSelectedTool.color },
                }
              : sq
          )
        );
        return;
      }

      // No tool selected: select to reposition pieces
      const clickedSq = board.find((s) => s.row === row && s.col === col);
      if (selectedSquare) {
        const [selR, selC] = selectedSquare;
        if (selR === row && selC === col) {
          setSelectedSquare(null);
          return;
        }

        const sourceSq = board.find((s) => s.row === selR && s.col === selC);
        const pieceToMove = sourceSq?.piece ?? null;

        setBoard((prev) =>
          prev.map((sq) => {
            if (sq.row === selR && sq.col === selC) {
              return { ...sq, piece: null };
            }
            if (sq.row === row && sq.col === col) {
              return { ...sq, piece: pieceToMove };
            }
            return sq;
          })
        );
        setSelectedSquare(null);
      } else {
        if (clickedSq?.piece) {
          setSelectedSquare([row, col]);
        }
      }
    },
    [puzzleSelectedTool, board, selectedSquare]
  );

  // Right-click in Setup Mode: erase piece
  const handleSetupSquareRightClick = useCallback((row: number, col: number) => {
    setBoard((prev) =>
      prev.map((sq) => (sq.row === row && sq.col === col ? { ...sq, piece: null } : sq))
    );
    setSelectedSquare(null);
  }, []);

  // Start Puzzle Solving
  const handleStartSolving = useCallback(() => {
    try {
      const fen = boardToFen(board, puzzleFirstTurn);
      const newState = WasmGameState.from_fen(fen);
      gameStateRef.current = newState;
      setInitialPuzzleFen(fen);
      setPuzzleStage('playing');
      syncGameState();
      setSelectedSquare(null);
      setLegalMoves([]);
      setLastMove(null);
      setLastStats(null);
      setHistory([]);
      if (puzzleSubMode === 'ComputerVsComputer') {
        setIsPaused(false);
      }
    } catch (err) {
      console.error('Failed to start puzzle solving:', err);
    }
  }, [board, puzzleFirstTurn, puzzleSubMode, syncGameState]);

  // Restart Puzzle from Beginning
  const handleRestartPuzzle = useCallback(() => {
    if (isThinking || !initialPuzzleFen) return;
    try {
      const newState = WasmGameState.from_fen(initialPuzzleFen);
      gameStateRef.current = newState;
      syncGameState();
      setSelectedSquare(null);
      setLegalMoves([]);
      setLastMove(null);
      setLastStats(null);
      setHistory([]);
      if (puzzleSubMode === 'ComputerVsComputer') {
        setIsPaused(false);
      }
    } catch (err) {
      console.error('Failed to restart puzzle:', err);
    }
  }, [initialPuzzleFen, isThinking, puzzleSubMode, syncGameState]);

  // Edit Puzzle Position (back to setup)
  const handleEditPuzzlePosition = useCallback(() => {
    if (isThinking) return;
    setPuzzleStage('setup');
    setSelectedSquare(null);
    setLegalMoves([]);
    setLastMove(null);
    if (initialPuzzleFen) {
      try {
        const { squares, turn: initialTurn } = fenToBoard(initialPuzzleFen);
        setBoard(squares);
        setPuzzleFirstTurn(initialTurn);
      } catch (e) {
        // keep current board
      }
    }
  }, [initialPuzzleFen, isThinking]);

  // Handle Game Mode Change
  const handleGameModeChange = useCallback(
    (mode: GameMode) => {
      if (isThinking) return;
      setGameMode(mode);
      setSelectedSquare(null);
      setLegalMoves([]);
      setLastMove(null);
      setLastStats(null);

      if (mode === 'Puzzle') {
        setPuzzleStage('setup');
        const defaultPuzzle = CLASSIC_PUZZLES[0]!;
        try {
          const { squares, turn: presetTurn } = fenToBoard(defaultPuzzle.fen);
          setBoard(squares);
          setPuzzleFirstTurn(presetTurn);
          setInitialPuzzleFen(defaultPuzzle.fen);
        } catch (e) {
          // fallback
        }
      } else {
        gameStateRef.current = new WasmGameState();
        syncGameState();
        if (mode === 'ComputerVsComputer') {
          setIsPaused(true);
        } else {
          setIsPaused(false);
        }
      }
    },
    [isThinking, syncGameState]
  );

  // Handle Board Click
  const handleSquareClick = (row: number, col: number) => {
    if (gameMode === 'Puzzle' && puzzleStage === 'setup') {
      handleSetupSquareClick(row, col);
      return;
    }

    if (isThinking || status.status !== 'Playing') return;
    const state = gameStateRef.current;
    if (!state) return;

    const effectivePlayMode =
      gameMode === 'Puzzle' ? puzzleSubMode : gameMode;

    if (effectivePlayMode === 'HumanVsComputer' && turn !== playerSide) return;
    if (effectivePlayMode === 'ComputerVsComputer') return;

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
    if (gameMode === 'Puzzle') {
      handleRestartPuzzle();
      return;
    }
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

    const effectivePlayMode =
      gameMode === 'Puzzle' ? puzzleSubMode : gameMode;

    if (effectivePlayMode === 'HumanVsComputer') {
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

  const topSide: PieceColor = playerSide === 'red' ? 'black' : 'red';
  const bottomSide: PieceColor = playerSide === 'red' ? 'red' : 'black';

  const getSideData = (s: PieceColor) => {
    if (s === 'red') {
      return {
        side: 'red' as PieceColor,
        isTurn: turn === 'red' && status.status === 'Playing',
        isThinking: isThinking && turn === 'red',
        capturedPieces: capturedSummary.groupedBlackCaptured,
        totalCapturedValue: capturedSummary.blackCapturedValue,
        materialAdvantage: capturedSummary.advantageForRed,
      };
    } else {
      return {
        side: 'black' as PieceColor,
        isTurn: turn === 'black' && status.status === 'Playing',
        isThinking: isThinking && turn === 'black',
        capturedPieces: capturedSummary.groupedRedCaptured,
        totalCapturedValue: capturedSummary.redCapturedValue,
        materialAdvantage: -capturedSummary.advantageForRed,
      };
    }
  };

  const topData = getSideData(topSide);
  const bottomData = getSideData(bottomSide);

  return (
    <div className="min-h-screen bg-[#14110F] text-[#F5EFE6] flex flex-col justify-between py-3 sm:py-5">
      {/* Top Bar */}
      <header
        style={{
          maxWidth:
            layoutOrientation === 'sides'
              ? `${Math.max(900, boardSize + 390)}px`
              : `${Math.max(340, boardSize + 24)}px`,
        }}
        className="w-full mx-auto px-3 sm:px-6 flex items-center justify-between gap-3 sm:gap-4 pb-3 mb-2 sm:mb-3 border-b border-stone-800/80 transition-[max-width] duration-150"
      >
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

        {/* Layout Switcher & Active Turn & Check Status Indicator */}
        <div className="flex items-center gap-2 sm:gap-3">
          {/* Quick Layout Mode Switcher */}
          <div className="hidden sm:flex items-center bg-[#1E1915] border border-stone-800 rounded-xl p-0.5 text-xs">
            <button
              type="button"
              onClick={() => setLayoutOverride('auto')}
              className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
                layoutOverride === 'auto'
                  ? 'bg-amber-500/25 text-amber-200 font-semibold'
                  : 'text-stone-400 hover:text-stone-200'
              }`}
              title="Tự động theo tỉ lệ màn hình"
            >
              <LayoutGrid className="w-3.5 h-3.5" />
              <span className="text-[11px]">Tự động</span>
            </button>
            <button
              type="button"
              onClick={() => setLayoutOverride('sides')}
              className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
                layoutOverride === 'sides'
                  ? 'bg-amber-500/25 text-amber-200 font-semibold'
                  : 'text-stone-400 hover:text-stone-200'
              }`}
              title="Hai bên (Trái - Phải)"
            >
              <Columns className="w-3.5 h-3.5" />
              <span className="text-[11px]">Hai bên</span>
            </button>
            <button
              type="button"
              onClick={() => setLayoutOverride('stacked')}
              className={`inline-flex items-center gap-1 px-2 py-1 rounded-lg transition-colors cursor-pointer ${
                layoutOverride === 'stacked'
                  ? 'bg-amber-500/25 text-amber-200 font-semibold'
                  : 'text-stone-400 hover:text-stone-200'
              }`}
              title="Trên - Dưới"
            >
              <Rows className="w-3.5 h-3.5" />
              <span className="text-[11px]">Trên dưới</span>
            </button>
          </div>

          {isInCheck && status.status === 'Playing' && (
            <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold">
              <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
              <span className="hidden xs:inline">Chiếu tướng!</span>
            </div>
          )}

          <div className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-[#1E1915] border border-stone-800">
            <span
              className={`w-5 h-5 rounded-full flex items-center justify-center font-display-serif text-xs font-bold ${
                (gameMode === 'Puzzle' && puzzleStage === 'setup' ? puzzleFirstTurn : turn) === 'red'
                  ? 'bg-red-900 text-red-200 border border-red-500/50'
                  : 'bg-stone-800 text-stone-200 border border-stone-600'
              }`}
            >
              {(gameMode === 'Puzzle' && puzzleStage === 'setup' ? puzzleFirstTurn : turn) === 'red'
                ? '帥'
                : '將'}
            </span>
            <div className="text-xs">
              <span className="text-stone-400 hidden xs:inline">
                {gameMode === 'Puzzle' && puzzleStage === 'setup' ? 'Đi tiên: ' : 'Lượt đi: '}
              </span>
              <span
                className={
                  (gameMode === 'Puzzle' && puzzleStage === 'setup' ? puzzleFirstTurn : turn) === 'red'
                    ? 'font-semibold text-red-400'
                    : 'font-semibold text-amber-300'
                }
              >
                {(gameMode === 'Puzzle' && puzzleStage === 'setup' ? puzzleFirstTurn : turn) === 'red'
                  ? 'Bên Đỏ'
                  : 'Bên Đen'}
              </span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col items-center justify-center gap-3 sm:gap-4 w-full mx-auto">
        <div
          style={{
            maxWidth:
              layoutOrientation === 'sides'
                ? `${Math.max(900, boardSize + 390)}px`
                : `${Math.max(340, boardSize + 24)}px`,
          }}
          className="w-full px-3 sm:px-6 transition-[max-width] duration-150"
        >
          {gameMode === 'Puzzle' && puzzleStage === 'setup' ? (
            <PuzzleSetupArea
              board={board}
              setBoard={setBoard}
              firstTurn={puzzleFirstTurn}
              setFirstTurn={setPuzzleFirstTurn}
              subMode={puzzleSubMode}
              setSubMode={setPuzzleSubMode}
              playerSide={playerSide}
              setPlayerSide={setPlayerSide}
              difficulty={difficulty}
              setDifficulty={setDifficulty}
              selectedTool={puzzleSelectedTool}
              setSelectedTool={setPuzzleSelectedTool}
              onStartSolving={handleStartSolving}
              onExitPuzzleMode={() => handleGameModeChange('HumanVsComputer')}
            />
          ) : (
            <ControlsArea
              gameMode={gameMode}
              setGameMode={handleGameModeChange}
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
              puzzleStage={puzzleStage}
              puzzleSubMode={puzzleSubMode}
              onEditPuzzlePosition={handleEditPuzzlePosition}
              onRestartPuzzle={handleRestartPuzzle}
            />
          )}
        </div>

        {/* Board & Captured Pieces Container: Responsive Sides or Stacked Layout */}
        {layoutOrientation === 'sides' ? (
          /* Landscape mode: Left tray | Board | Right tray */
          <div
            style={{ maxWidth: `${boardSize + 390}px` }}
            className="w-full px-2 sm:px-4 flex flex-row items-center justify-center gap-3 sm:gap-5 my-1 pb-1 sm:pb-2 transition-all"
          >
            {/* Left Tray: Opponent / Top Side */}
            <div style={{ height: `${boardSize}px` }} className="flex">
              <CapturedPiecesTray
                side={topData.side}
                isTurn={topData.isTurn}
                isThinking={topData.isThinking}
                capturedPieces={topData.capturedPieces}
                totalCapturedValue={topData.totalCapturedValue}
                materialAdvantage={topData.materialAdvantage}
                gameMode={gameMode}
                playerSide={playerSide}
                difficulty={difficulty}
                orientation="sides"
              />
            </div>

            {/* Center Board */}
            <div className="relative flex flex-col items-center justify-center shrink-0">
              <BoardCanvas
                board={board}
                playerSide={playerSide}
                selectedSquare={selectedSquare}
                legalMoves={legalMoves}
                lastMove={lastMove}
                onSquareClick={handleSquareClick}
                onSquareRightClick={handleSetupSquareRightClick}
                disabled={isThinking || (status.status !== 'Playing' && !(gameMode === 'Puzzle' && puzzleStage === 'setup'))}
                size={boardSize}
                isSetupMode={gameMode === 'Puzzle' && puzzleStage === 'setup'}
              >
                <ThinkingIndicator
                  isThinking={isThinking}
                  turn={turn}
                  difficulty={difficulty}
                  lastStats={lastStats}
                />
                {status.status !== 'Playing' && !(gameMode === 'Puzzle' && puzzleStage === 'setup') && (
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
                            {gameMode === 'Puzzle'
                              ? `Giải cờ thế thành công sau ${history.length} nước!`
                              : `Chiếu bí (Checkmate) sau ${history.length} nước đi`}
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

                      {gameMode === 'Puzzle' ? (
                        <div className="mt-4 flex flex-col gap-2">
                          <button
                            type="button"
                            onClick={handleRestartPuzzle}
                            className="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-colors cursor-pointer shadow-md"
                          >
                            <RotateCcw className="w-4 h-4" />
                            <span>Giải lại từ đầu</span>
                          </button>
                          <button
                            type="button"
                            onClick={handleEditPuzzlePosition}
                            className="w-full inline-flex items-center justify-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-[#26201C] hover:bg-stone-800 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
                          >
                            <span>Xếp lại cờ thế</span>
                          </button>
                        </div>
                      ) : (
                        <button
                          type="button"
                          onClick={handleNewGame}
                          className="mt-4 w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] text-white transition-colors cursor-pointer shadow-md"
                        >
                          <RotateCcw className="w-4 h-4" />
                          <span>Chơi ván mới</span>
                        </button>
                      )}
                    </div>
                  </div>
                )}
              </BoardCanvas>
            </div>

            {/* Right Tray: Player / Bottom Side */}
            <div style={{ height: `${boardSize}px` }} className="flex">
              <CapturedPiecesTray
                side={bottomData.side}
                isTurn={bottomData.isTurn}
                isThinking={bottomData.isThinking}
                capturedPieces={bottomData.capturedPieces}
                totalCapturedValue={bottomData.totalCapturedValue}
                materialAdvantage={bottomData.materialAdvantage}
                gameMode={gameMode}
                playerSide={playerSide}
                difficulty={difficulty}
                orientation="sides"
              />
            </div>
          </div>
        ) : (
          /* Portrait mode: Top tray (Opponent) | Board | Bottom tray (Player) */
          <div
            style={{ maxWidth: `${boardSize + 24}px` }}
            className="w-full px-2 sm:px-3 flex flex-col items-center justify-center gap-2 my-1 pb-2 sm:pb-3 transition-all"
          >
            {/* Top Tray: Opponent */}
            <div style={{ width: `${boardSize}px`, maxWidth: '100%' }}>
              <CapturedPiecesTray
                side={topData.side}
                isTurn={topData.isTurn}
                isThinking={topData.isThinking}
                capturedPieces={topData.capturedPieces}
                totalCapturedValue={topData.totalCapturedValue}
                materialAdvantage={topData.materialAdvantage}
                gameMode={gameMode}
                playerSide={playerSide}
                difficulty={difficulty}
                orientation="stacked"
              />
            </div>

            {/* Center Board */}
            <div className="relative flex flex-col items-center justify-center shrink-0">
              <BoardCanvas
                board={board}
                playerSide={playerSide}
                selectedSquare={selectedSquare}
                legalMoves={legalMoves}
                lastMove={lastMove}
                onSquareClick={handleSquareClick}
                onSquareRightClick={handleSetupSquareRightClick}
                disabled={isThinking || (status.status !== 'Playing' && !(gameMode === 'Puzzle' && puzzleStage === 'setup'))}
                size={boardSize}
                isSetupMode={gameMode === 'Puzzle' && puzzleStage === 'setup'}
              >
                <ThinkingIndicator
                  isThinking={isThinking}
                  turn={turn}
                  difficulty={difficulty}
                  lastStats={lastStats}
                />

                {status.status !== 'Playing' && !(gameMode === 'Puzzle' && puzzleStage === 'setup') && (
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
                            {gameMode === 'Puzzle'
                              ? `Giải cờ thế thành công sau ${history.length} nước!`
                              : `Chiếu bí (Checkmate) sau ${history.length} nước đi`}
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

                      {gameMode === 'Puzzle' ? (
                        <div className="mt-4 flex flex-col gap-2">
                          <button
                            type="button"
                            onClick={handleRestartPuzzle}
                            className="w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-colors cursor-pointer shadow-md"
                          >
                            <RotateCcw className="w-4 h-4" />
                            <span>Giải lại từ đầu</span>
                          </button>
                          <button
                            type="button"
                            onClick={handleEditPuzzlePosition}
                            className="w-full inline-flex items-center justify-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-[#26201C] hover:bg-stone-800 text-stone-200 border border-stone-700 transition-colors cursor-pointer"
                          >
                            <span>Xếp lại cờ thế</span>
                          </button>
                        </div>
                      ) : (
                        <button
                          type="button"
                          onClick={handleNewGame}
                          className="mt-4 w-full inline-flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl text-xs font-semibold bg-[#B91C1C] hover:bg-[#DC2626] text-white transition-colors cursor-pointer shadow-md"
                        >
                          <RotateCcw className="w-4 h-4" />
                          <span>Chơi ván mới</span>
                        </button>
                      )}
                    </div>
                  </div>
                )}
              </BoardCanvas>
            </div>

            {/* Bottom Tray: Player */}
            <div style={{ width: `${boardSize}px`, maxWidth: '100%' }}>
              <CapturedPiecesTray
                side={bottomData.side}
                isTurn={bottomData.isTurn}
                isThinking={bottomData.isThinking}
                capturedPieces={bottomData.capturedPieces}
                totalCapturedValue={bottomData.totalCapturedValue}
                materialAdvantage={bottomData.materialAdvantage}
                gameMode={gameMode}
                playerSide={playerSide}
                difficulty={difficulty}
                orientation="stacked"
              />
            </div>
          </div>
        )}

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
