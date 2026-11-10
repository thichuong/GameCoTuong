import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { Trophy, Handshake, AlertTriangle, RotateCcw, LayoutGrid, Columns, Rows } from 'lucide-react';
import init, { WasmGameState } from './wasm/cotuong_core.js';
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
import { BoardCanvas, getPieceCursorStyle } from './components/BoardCanvas.js';
import { ControlsArea } from './components/ControlsArea.js';
import { LogPanel } from './components/LogPanel.js';
import { ThinkingIndicator } from './components/ThinkingIndicator.js';
import { ConfigModal } from './components/ConfigModal.js';
import { CapturedPiecesTray } from './components/CapturedPiecesTray.js';
import { AiPoolManager } from './workers/aiPoolManager.js';
import {
  extractCapturedPieces,
  extractOffBoardPieces,
  determineLayoutOrientation,
  calculateOptimalBoardSize,
  STANDARD_PIECE_COUNTS,
  type LayoutOrientation,
} from './utils/capturedPieces.js';
import { soundManager } from './utils/sound.js';
import { exportCsv } from './utils/export.js';
import {
  boardToFen,
  fenToBoard,
  createTwoGeneralsBoard,
  TWO_GENERALS_FEN,
  STANDARD_BOARD_FEN,
} from './utils/fen.js';
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
  const [selectedSetupPiece, setSelectedSetupPiece] = useState<Piece | null>(null);
  const [draggedTrayPiece, setDraggedTrayPiece] = useState<Piece | null>(null);
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

  // Extract captured / off-board pieces summary:
  // In Puzzle mode, trays always reflect off-board pieces from the standard 32-piece set
  const capturedSummary = useMemo(
    () =>
      gameMode === 'Puzzle'
        ? extractOffBoardPieces(board)
        : extractCapturedPieces(history),
    [gameMode, board, history]
  );

  // Clear selectedSetupPiece if no more of that piece remain in the off-board tray
  useEffect(() => {
    if (!selectedSetupPiece) return;
    const countOnBoard = board.filter(
      (sq) =>
        sq.piece?.type === selectedSetupPiece.type &&
        sq.piece?.color === selectedSetupPiece.color
    ).length;
    if (countOnBoard >= STANDARD_PIECE_COUNTS[selectedSetupPiece.type]) {
      setSelectedSetupPiece(null);
    }
  }, [board, selectedSetupPiece]);

  // AI Multi-threaded Web Worker Pool
  const aiPoolRef = useRef<AiPoolManager | null>(null);

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

  // Clear AI engine transposition table & memory when explicitly starting a fresh game or puzzle
  const clearEngineMemory = useCallback(() => {
    aiPoolRef.current?.clearMemory();
  }, []);

  // Initialize WASM and AI Worker Pool
  useEffect(() => {
    async function setup() {
      await init();
      gameStateRef.current = new WasmGameState();
      syncGameState();
      setIsWasmReady(true);

      const pool = new AiPoolManager(DEFAULT_ENGINE_CONFIG.threads);
      aiPoolRef.current = pool;
    }

    setup().catch((err) => {
      console.error('Failed to initialize WASM or AI worker pool:', err);
    });

    return () => {
      aiPoolRef.current?.terminate();
    };
  }, [syncGameState]);

  // AI Search Trigger
  const triggerAiMove = useCallback(() => {
    if (isAiProcessingRef.current) return;
    const state = gameStateRef.current;
    const pool = aiPoolRef.current;
    if (!state || !pool) return;

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

    // 2. Offload to Multi-threaded AI Worker Pool
    const currentTurn = state.turn() as PieceColor;
    const config = currentTurn === 'red' ? redConfigRef.current : blackConfigRef.current;
    const timeLimitMs = DIFFICULTY_TIMES[difficultyRef.current];
    const threadCount = config.threads || DEFAULT_ENGINE_CONFIG.threads || 1;

    pool
      .search({
        state,
        stateJson: state.to_json(),
        fen: state.to_fen(),
        timeLimitMs,
        configJson: JSON.stringify(config),
        threads: threadCount,
      })
      .then((result) => {
        if (!result) return;
        const { move, stats } = result;

        try {
          const res = state.make_move(
            move.from_row,
            move.from_col,
            move.to_row,
            move.to_col
          ) as { success: boolean; captured: unknown; in_check: boolean };

          const nps = stats.time_ms > 0 ? Math.round((stats.nodes / stats.time_ms) * 1000) : 0;
          const npsStr = nps >= 1000 ? `${(nps / 1000).toFixed(1)}k` : `${nps}`;
          const threadStr = threadCount > 1 ? ` [${threadCount} luồng]` : '';
          state.annotate_last_move(
            `🤖 Độ sâu: ${stats.depth}, Nodes: ${stats.nodes}, Thời gian: ${stats.time_ms}ms (${npsStr} NPS)${threadStr}`
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
        }
      })
      .catch((err) => {
        console.error('Error during AI search in pool:', err);
      })
      .finally(() => {
        isAiProcessingRef.current = false;
        setIsThinking(false);
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

  // Drop piece from CapturedPiecesTray onto Board square in Setup Mode
  const handleDropPieceOnSquare = useCallback(
    (row: number, col: number, piece: Piece) => {
      setBoard((prev) => {
        const currentCount = prev.filter(
          (s) =>
            !(s.row === row && s.col === col) &&
            s.piece?.type === piece.type &&
            s.piece?.color === piece.color
        ).length;
        if (currentCount >= STANDARD_PIECE_COUNTS[piece.type]) {
          return prev;
        }
        return prev.map((sq) =>
          sq.row === row && sq.col === col
            ? { ...sq, piece: { type: piece.type, color: piece.color } }
            : sq
        );
      });
      setSelectedSquare(null);
      setDraggedTrayPiece(null);
      soundManager.play('move');
    },
    []
  );

  // Move piece on Board via Left-Click Drag in Setup Mode
  const handleMoveSetupPiece = useCallback(
    (fromRow: number, fromCol: number, toRow: number, toCol: number) => {
      if (fromRow === toRow && fromCol === toCol) return;
      setBoard((prev) => {
        const sourceSq = prev.find((s) => s.row === fromRow && s.col === fromCol);
        const pieceToMove = sourceSq?.piece ?? null;
        if (!pieceToMove) return prev;

        return prev.map((sq) => {
          if (sq.row === fromRow && sq.col === fromCol) {
            return { ...sq, piece: null };
          }
          if (sq.row === toRow && sq.col === toCol) {
            return { ...sq, piece: pieceToMove };
          }
          return sq;
        });
      });
      setSelectedSquare(null);
      soundManager.play('move');
    },
    []
  );

  // Handle Board Click in Setup Mode (supports click-to-place from tray or click-to-move on board)
  const handleSetupSquareClick = useCallback(
    (row: number, col: number) => {
      if (selectedSetupPiece) {
        handleDropPieceOnSquare(row, col, selectedSetupPiece);
        return;
      }

      const clickedSq = board.find((s) => s.row === row && s.col === col);
      if (selectedSquare) {
        const [selR, selC] = selectedSquare;
        if (selR === row && selC === col) {
          setSelectedSquare(null);
          return;
        }

        handleMoveSetupPiece(selR, selC, row, col);
      } else {
        if (clickedSq?.piece) {
          setSelectedSquare([row, col]);
        }
      }
    },
    [selectedSetupPiece, board, selectedSquare, handleDropPieceOnSquare, handleMoveSetupPiece]
  );

  // Select piece from CapturedPiecesTray in Setup Mode
  const handleSelectSetupPiece = useCallback((piece: Piece | null) => {
    setSelectedSetupPiece(piece);
    if (piece) {
      setSelectedSquare(null);
    }
  }, []);

  // Right-click in Setup Mode: erase piece (returns to opponent's captured tray) and clear selection
  const handleSetupSquareRightClick = useCallback((row: number, col: number) => {
    setBoard((prev) =>
      prev.map((sq) => (sq.row === row && sq.col === col ? { ...sq, piece: null } : sq))
    );
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
  }, []);

  // Update Puzzle Board (from Preset or FEN modal)
  const handleUpdatePuzzleBoard = useCallback(
    (squares: Square[], nextTurn?: PieceColor, recommendedMode?: PuzzleSubMode) => {
      setBoard(squares);
      if (nextTurn) {
        setPuzzleFirstTurn(nextTurn);
      }
      if (recommendedMode) {
        setPuzzleSubMode(recommendedMode);
      }
      setSelectedSquare(null);
      setSelectedSetupPiece(null);
    },
    []
  );

  // Clear Puzzle Board (keep 2 Generals at their default palace positions)
  const handleClearPuzzleBoard = useCallback(() => {
    setBoard(createTwoGeneralsBoard());
    setPuzzleFirstTurn('red');
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
  }, []);

  // Reset Puzzle Board to Standard 32 pieces
  const handleResetStandardBoard = useCallback(() => {
    const { squares } = fenToBoard(STANDARD_BOARD_FEN);
    setBoard(squares);
    setPuzzleFirstTurn('red');
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
  }, []);

  // Start Puzzle Solving
  const handleStartSolving = useCallback(() => {
    try {
      clearEngineMemory();
      const fen = boardToFen(board, puzzleFirstTurn);
      const newState = WasmGameState.from_fen(fen);
      gameStateRef.current = newState;
      setInitialPuzzleFen(fen);
      setPuzzleStage('playing');
      syncGameState();
      setSelectedSquare(null);
      setSelectedSetupPiece(null);
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
  }, [board, clearEngineMemory, puzzleFirstTurn, puzzleSubMode, syncGameState]);

  // Restart Puzzle from Beginning
  const handleRestartPuzzle = useCallback(() => {
    if (isThinking || !initialPuzzleFen) return;
    try {
      clearEngineMemory();
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
  }, [clearEngineMemory, initialPuzzleFen, isThinking, puzzleSubMode, syncGameState]);

  // Edit Puzzle Position (back to setup)
  const handleEditPuzzlePosition = useCallback(() => {
    if (isThinking) return;
    setPuzzleStage('setup');
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
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
      clearEngineMemory();
      setGameMode(mode);
      setSelectedSquare(null);
      setSelectedSetupPiece(null);
      setLegalMoves([]);
      setLastMove(null);
      setLastStats(null);

      if (mode === 'Puzzle') {
        setPuzzleStage('setup');
        setBoard(createTwoGeneralsBoard());
        setPuzzleFirstTurn('red');
        setInitialPuzzleFen(TWO_GENERALS_FEN);
        setHistory([]);
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
    [clearEngineMemory, isThinking, syncGameState]
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
    clearEngineMemory();
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

  const isPuzzleSetup = gameMode === 'Puzzle' && puzzleStage === 'setup';
  const topSide: PieceColor = playerSide === 'red' ? 'black' : 'red';
  const bottomSide: PieceColor = playerSide === 'red' ? 'red' : 'black';

  // Active selected piece (either from tray in setup mode or selected square on board)
  const activeCursorPiece = useMemo(() => {
    if (isPuzzleSetup && selectedSetupPiece) return selectedSetupPiece;
    if (selectedSquare) {
      const sq = board.find((s) => s.row === selectedSquare[0] && s.col === selectedSquare[1]);
      return sq?.piece ?? null;
    }
    return null;
  }, [isPuzzleSetup, selectedSetupPiece, selectedSquare, board]);

  const activePieceCursorStyle = useMemo(
    () => getPieceCursorStyle(activeCursorPiece),
    [activeCursorPiece]
  );

  const getSideData = (s: PieceColor) => {
    if (s === 'red') {
      return {
        side: 'red' as PieceColor,
        isTurn: !isPuzzleSetup && turn === 'red' && status.status === 'Playing',
        isThinking: isThinking && turn === 'red',
        capturedPieces: capturedSummary.groupedBlackCaptured,
        totalCapturedValue: capturedSummary.blackCapturedValue,
        materialAdvantage: capturedSummary.advantageForRed,
      };
    } else {
      return {
        side: 'black' as PieceColor,
        isTurn: !isPuzzleSetup && turn === 'black' && status.status === 'Playing',
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
    <div
      style={activePieceCursorStyle ? { cursor: activePieceCursorStyle } : undefined}
      className="min-h-screen bg-[#14110F] text-[#F5EFE6] flex flex-col justify-between py-3 sm:py-5"
    >
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

          {isInCheck && status.status === 'Playing' && !isPuzzleSetup && (
            <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold">
              <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0" />
              <span className="hidden xs:inline">Chiếu tướng!</span>
            </div>
          )}

          <div className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-[#1E1915] border border-stone-800">
            <span
              className={`w-5 h-5 rounded-full flex items-center justify-center font-display-serif text-xs font-bold ${
                (isPuzzleSetup ? puzzleFirstTurn : turn) === 'red'
                  ? 'bg-red-900 text-red-200 border border-red-500/50'
                  : 'bg-stone-800 text-stone-200 border border-stone-600'
              }`}
            >
              {(isPuzzleSetup ? puzzleFirstTurn : turn) === 'red'
                ? '帥'
                : '將'}
            </span>
            <div className="text-xs">
              <span className="text-stone-400 hidden xs:inline">
                {isPuzzleSetup ? 'Đi tiên: ' : 'Lượt đi: '}
              </span>
              <span
                className={
                  (isPuzzleSetup ? puzzleFirstTurn : turn) === 'red'
                    ? 'font-semibold text-red-400'
                    : 'font-semibold text-amber-300'
                }
              >
                {(isPuzzleSetup ? puzzleFirstTurn : turn) === 'red'
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
            setPuzzleSubMode={setPuzzleSubMode}
            puzzleFirstTurn={puzzleFirstTurn}
            setPuzzleFirstTurn={setPuzzleFirstTurn}
            board={board}
            onUpdatePuzzleBoard={handleUpdatePuzzleBoard}
            onClearPuzzleBoard={handleClearPuzzleBoard}
            onResetStandardBoard={handleResetStandardBoard}
            onStartSolving={handleStartSolving}
            onEditPuzzlePosition={handleEditPuzzlePosition}
            onRestartPuzzle={handleRestartPuzzle}
          />
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
                isSetupMode={isPuzzleSetup}
                selectedSetupPiece={selectedSetupPiece}
                onSelectSetupPiece={handleSelectSetupPiece}
                onDragStartSetupPiece={setDraggedTrayPiece}
                onDragEndSetupPiece={() => setDraggedTrayPiece(null)}
              />
            </div>

            {/* Center Board */}
            <div className="relative flex flex-col items-center justify-center shrink-0">
              <BoardCanvas
                board={board}
                playerSide={playerSide}
                selectedSquare={selectedSquare}
                selectedSetupPiece={selectedSetupPiece}
                legalMoves={legalMoves}
                lastMove={lastMove}
                onSquareClick={handleSquareClick}
                onSquareRightClick={handleSetupSquareRightClick}
                onDropPieceOnSquare={handleDropPieceOnSquare}
                onMoveSetupPiece={handleMoveSetupPiece}
                externalDraggedPiece={draggedTrayPiece}
                disabled={isThinking || (status.status !== 'Playing' && !isPuzzleSetup)}
                size={boardSize}
                isSetupMode={isPuzzleSetup}
              >
                <ThinkingIndicator
                  isThinking={isThinking}
                  turn={turn}
                  difficulty={difficulty}
                  lastStats={lastStats}
                />
                {status.status !== 'Playing' && !isPuzzleSetup && (
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
                isSetupMode={isPuzzleSetup}
                selectedSetupPiece={selectedSetupPiece}
                onSelectSetupPiece={handleSelectSetupPiece}
                onDragStartSetupPiece={setDraggedTrayPiece}
                onDragEndSetupPiece={() => setDraggedTrayPiece(null)}
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
                isSetupMode={isPuzzleSetup}
                selectedSetupPiece={selectedSetupPiece}
                onSelectSetupPiece={handleSelectSetupPiece}
                onDragStartSetupPiece={setDraggedTrayPiece}
                onDragEndSetupPiece={() => setDraggedTrayPiece(null)}
              />
            </div>

            {/* Center Board */}
            <div className="relative flex flex-col items-center justify-center shrink-0">
              <BoardCanvas
                board={board}
                playerSide={playerSide}
                selectedSquare={selectedSquare}
                selectedSetupPiece={selectedSetupPiece}
                legalMoves={legalMoves}
                lastMove={lastMove}
                onSquareClick={handleSquareClick}
                onSquareRightClick={handleSetupSquareRightClick}
                onDropPieceOnSquare={handleDropPieceOnSquare}
                onMoveSetupPiece={handleMoveSetupPiece}
                externalDraggedPiece={draggedTrayPiece}
                disabled={isThinking || (status.status !== 'Playing' && !isPuzzleSetup)}
                size={boardSize}
                isSetupMode={isPuzzleSetup}
              >
                <ThinkingIndicator
                  isThinking={isThinking}
                  turn={turn}
                  difficulty={difficulty}
                  lastStats={lastStats}
                />

                {status.status !== 'Playing' && !isPuzzleSetup && (
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
                isSetupMode={isPuzzleSetup}
                selectedSetupPiece={selectedSetupPiece}
                onSelectSetupPiece={handleSelectSetupPiece}
                onDragStartSetupPiece={setDraggedTrayPiece}
                onDragEndSetupPiece={() => setDraggedTrayPiece(null)}
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
