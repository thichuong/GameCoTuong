import { useState, useCallback } from 'react';
import type { BoardViewMode } from '../types/game.js';

export const BOARD_VIEW_MODE_STORAGE_KEY = 'xiangqi_board_view_mode';

/**
 * Parses a raw string value into a valid BoardViewMode, defaulting to '2.5d'.
 */
export function parseBoardViewMode(raw: string | null | undefined): BoardViewMode {
  if (raw === '2d' || raw === '2.5d' || raw === '3d') {
    return raw;
  }
  return '2.5d';
}

/**
 * Reads the persisted board view mode from localStorage safely.
 */
export function loadStoredBoardViewMode(): BoardViewMode {
  if (typeof window === 'undefined') {
    return '2.5d';
  }
  try {
    return parseBoardViewMode(window.localStorage.getItem(BOARD_VIEW_MODE_STORAGE_KEY));
  } catch {
    return '2.5d';
  }
}

/**
 * Persists the active board view mode to localStorage safely.
 */
export function saveStoredBoardViewMode(mode: BoardViewMode): void {
  if (typeof window === 'undefined') {
    return;
  }
  try {
    window.localStorage.setItem(BOARD_VIEW_MODE_STORAGE_KEY, mode);
  } catch {
    // Ignore storage errors in restricted or private browsing contexts
  }
}

/**
 * React hook managing the 2D / 2.5D / 3D board view mode with localStorage persistence.
 */
export function useBoardViewMode() {
  const [boardViewMode, setBoardViewModeState] = useState<BoardViewMode>(() =>
    loadStoredBoardViewMode()
  );

  const setBoardViewMode = useCallback((mode: BoardViewMode) => {
    setBoardViewModeState(mode);
    saveStoredBoardViewMode(mode);
  }, []);

  return {
    boardViewMode,
    setBoardViewMode,
  };
}
