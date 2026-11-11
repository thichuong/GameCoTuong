import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import init, { WasmGameState } from '../wasm/cotuong_core.js';
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
} from '../types/game.js';
import { DEFAULT_ENGINE_CONFIG } from '../types/game.js';
import { AiPoolManager } from '../workers/aiPoolManager.js';
import {
  extractCapturedPieces,
  extractOffBoardPieces,
  STANDARD_PIECE_COUNTS,
} from '../utils/capturedPieces.js';
import { soundManager } from '../utils/sound.js';
import {
  boardToFen,
  fenToBoard,
  createTwoGeneralsBoard,
  TWO_GENERALS_FEN,
  STANDARD_BOARD_FEN,
} from '../utils/fen.js';

const DIFFICULTY_TIMES: Record<Difficulty, number> = {
  1: 1000,
  2: 2000,
  3: 5000,
  4: 10000,
  5: 20000,
};

export function useXiangqiGame() {
  const [isWasmReady, setIsWasmReady] = useState(false);
  const gameStateRef = useRef<WasmGameState | null>(null);
  const aiPoolRef = useRef<AiPoolManager | null>(null);

  // Game Settings & State
  const [gameMode, setGameMode] = useState<GameMode>('HumanVsComputer');
  const [playerSide, setPlayerSide] = useState<PieceColor>('red');
  const [difficulty, setDifficulty] = useState<Difficulty>(1);
  const [isPaused, setIsPaused] = useState(false);
  const [isThinking, setIsThinking] = useState(false);
  const [lastStats, setLastStats] = useState<SearchStats | null>(null);

  // Puzzle Mode State
  const [puzzleStage, setPuzzleStage] = useState<PuzzleStage>('setup');
  const [puzzleSubMode, setPuzzleSubMode] =
    useState<PuzzleSubMode>('HumanVsComputer');
  const [puzzleFirstTurn, setPuzzleFirstTurn] = useState<PieceColor>('red');
  const [selectedSetupPiece, setSelectedSetupPiece] = useState<Piece | null>(null);
  const [draggedTrayPiece, setDraggedTrayPiece] = useState<Piece | null>(null);
  const [initialPuzzleFen, setInitialPuzzleFen] = useState<string | null>(null);

  // Board presentation
  const [board, setBoard] = useState<Square[]>([]);
  const [turn, setTurn] = useState<PieceColor>('red');
  const [status, setStatus] = useState<GameStatus>({ status: 'Playing' });
  const [isInCheck, setIsInCheck] = useState(false);
  const [selectedSquare, setSelectedSquare] = useState<[number, number] | null>(
    null
  );
  const [legalMoves, setLegalMoves] = useState<[number, number][]>([]);
  const [lastMove, setLastMove] = useState<{
    from_row: number;
    from_col: number;
    to_row: number;
    to_col: number;
  } | null>(null);
  const [history, setHistory] = useState<HistoryRecord[]>([]);

  // Engine Config State
  const [redConfig, setRedConfig] = useState<EngineConfig>(DEFAULT_ENGINE_CONFIG);
  const [blackConfig, setBlackConfig] =
    useState<EngineConfig>(DEFAULT_ENGINE_CONFIG);

  const redConfigRef = useRef(redConfig);
  redConfigRef.current = redConfig;
  const blackConfigRef = useRef(blackConfig);
  blackConfigRef.current = blackConfig;
  const difficultyRef = useRef(difficulty);
  difficultyRef.current = difficulty;
  const isAiProcessingRef = useRef(false);

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

  const syncGameState = useCallback(() => {
    const state = gameStateRef.current;
    if (!state) return;

    setBoard(state.get_board() as Square[]);
    setTurn(state.turn() as PieceColor);
    setStatus(state.status() as GameStatus);
    setIsInCheck(state.is_in_check());
    setHistory(state.get_history() as HistoryRecord[]);
  }, []);

  const clearEngineMemory = useCallback(() => {
    aiPoolRef.current?.clearMemory();
  }, []);

  const handleEngineConfigChange = useCallback((newConfig: EngineConfig) => {
    aiPoolRef.current?.resetConfig(newConfig);
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

    const isPuzzle = gameMode === 'Puzzle';
    const rawBookMove = isPuzzle
      ? null
      : (state.get_book_move() as MoveCoords | null);
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

    const currentTurn = state.turn() as PieceColor;
    const config =
      currentTurn === 'red' ? redConfigRef.current : blackConfigRef.current;
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

          const nps =
            stats.time_ms > 0
              ? Math.round((stats.nodes / stats.time_ms) * 1000)
              : 0;
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
          try {
            const fallbackMoves =
              state.get_strictly_legal_moves_flat() as number[];
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
      gameMode === 'Puzzle'
        ? puzzleStage === 'playing'
          ? puzzleSubMode
          : null
        : gameMode;

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

  const handleMoveSetupPiece = useCallback(
    (fromRow: number, fromCol: number, toRow: number, toCol: number) => {
      if (fromRow === toRow && fromCol === toCol) return;
      setBoard((prev) => {
        const sourceSq = prev.find(
          (s) => s.row === fromRow && s.col === fromCol
        );
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
      } else if (clickedSq?.piece) {
        setSelectedSquare([row, col]);
      }
    },
    [
      selectedSetupPiece,
      board,
      selectedSquare,
      handleDropPieceOnSquare,
      handleMoveSetupPiece,
    ]
  );

  const handleSelectSetupPiece = useCallback((piece: Piece | null) => {
    setSelectedSetupPiece(piece);
    if (piece) {
      setSelectedSquare(null);
    }
  }, []);

  const handleSetupSquareRightClick = useCallback(
    (row: number, col: number) => {
      setBoard((prev) =>
        prev.map((sq) =>
          sq.row === row && sq.col === col ? { ...sq, piece: null } : sq
        )
      );
      setSelectedSquare(null);
      setSelectedSetupPiece(null);
    },
    []
  );

  const handleUpdatePuzzleBoard = useCallback(
    (
      squares: Square[],
      nextTurn?: PieceColor,
      recommendedMode?: PuzzleSubMode
    ) => {
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

  const handleClearPuzzleBoard = useCallback(() => {
    setBoard(createTwoGeneralsBoard());
    setPuzzleFirstTurn('red');
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
  }, []);

  const handleResetStandardBoard = useCallback(() => {
    const { squares } = fenToBoard(STANDARD_BOARD_FEN);
    setBoard(squares);
    setPuzzleFirstTurn('red');
    setSelectedSquare(null);
    setSelectedSetupPiece(null);
  }, []);

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
  }, [
    clearEngineMemory,
    initialPuzzleFen,
    isThinking,
    puzzleSubMode,
    syncGameState,
  ]);

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
      } catch {
        // keep current board
      }
    }
  }, [initialPuzzleFen, isThinking]);

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
        setIsPaused(mode === 'ComputerVsComputer');
      }
    },
    [clearEngineMemory, isThinking, syncGameState]
  );

  const handleSquareClick = useCallback(
    (row: number, col: number) => {
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

            setLastMove({
              from_row: selRow,
              from_col: selCol,
              to_row: row,
              to_col: col,
            });
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
    },
    [
      gameMode,
      puzzleStage,
      handleSetupSquareClick,
      isThinking,
      status.status,
      puzzleSubMode,
      turn,
      playerSide,
      board,
      selectedSquare,
      legalMoves,
      syncGameState,
    ]
  );

  const handleNewGame = useCallback(() => {
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
  }, [clearEngineMemory, gameMode, handleRestartPuzzle, isThinking, syncGameState]);

  const handleUndo = useCallback(() => {
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
  }, [gameMode, isThinking, playerSide, puzzleSubMode, syncGameState]);

  const isPuzzleSetup = gameMode === 'Puzzle' && puzzleStage === 'setup';

  const activeCursorPiece = useMemo(() => {
    if (isPuzzleSetup && selectedSetupPiece) return selectedSetupPiece;
    if (selectedSquare) {
      const sq = board.find(
        (s) => s.row === selectedSquare[0] && s.col === selectedSquare[1]
      );
      return sq?.piece ?? null;
    }
    return null;
  }, [isPuzzleSetup, selectedSetupPiece, selectedSquare, board]);

  return {
    gameMode,
    handleGameModeChange,
    playerSide,
    setPlayerSide,
    difficulty,
    setDifficulty,
    isPaused,
    setIsPaused,
    isThinking,
    lastStats,
    puzzleStage,
    puzzleSubMode,
    setPuzzleSubMode,
    puzzleFirstTurn,
    setPuzzleFirstTurn,
    selectedSetupPiece,
    draggedTrayPiece,
    setDraggedTrayPiece,
    board,
    turn,
    status,
    isInCheck,
    selectedSquare,
    legalMoves,
    lastMove,
    history,
    redConfig,
    setRedConfig,
    blackConfig,
    setBlackConfig,
    capturedSummary,
    isPuzzleSetup,
    activeCursorPiece,
    handleDropPieceOnSquare,
    handleMoveSetupPiece,
    handleSelectSetupPiece,
    handleSetupSquareRightClick,
    handleUpdatePuzzleBoard,
    handleClearPuzzleBoard,
    handleResetStandardBoard,
    handleStartSolving,
    handleRestartPuzzle,
    handleEditPuzzlePosition,
    handleSquareClick,
    handleNewGame,
    handleUndo,
    handleEngineConfigChange,
  };
}
