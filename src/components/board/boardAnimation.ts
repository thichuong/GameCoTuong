import type { Piece } from '../../types/game.js';
import type { CameraParams } from './boardProjection.js';

export const CAMERA_TRANSITION_MS = 320;
export const MOVE_ANIMATION_MS = 240;

export interface MoveAnimationState {
  fromRow: number;
  fromCol: number;
  toRow: number;
  toCol: number;
  piece: Piece;
  startTime: number;
  durationMs: number;
}

export interface CameraTransitionState {
  from: CameraParams;
  to: CameraParams;
  startTime: number;
  durationMs: number;
}

function clamp01(t: number): number {
  return Math.max(0, Math.min(1, t));
}

/**
 * Decelerating cubic easing for smooth camera transitions.
 */
export function easeOutCubic(t: number): number {
  const u = clamp01(t);
  return 1 - Math.pow(1 - u, 3);
}

/**
 * Smooth S-curve cubic easing for horizontal piece travel.
 */
export function easeInOutCubic(t: number): number {
  const u = clamp01(t);
  return u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2;
}

/**
 * Parabolic elevation profile peaking at t = 0.5 for 3D piece flight arcs.
 */
export function computeArcLiftZ(t: number, peakZ: number): number {
  const u = clamp01(t);
  return peakZ * 4 * u * (1 - u);
}

/**
 * Checks whether the user's system prefers reduced motion.
 */
export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') {
    return false;
  }
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}
