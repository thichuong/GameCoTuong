import { useState, useEffect, useMemo } from 'react';
import {
  determineLayoutOrientation,
  calculateOptimalBoardSize,
  type LayoutOrientation,
} from '../utils/capturedPieces.js';

export type LayoutOverride = 'auto' | 'sides' | 'stacked';

export function useResponsiveLayout() {
  const [layoutOverride, setLayoutOverride] = useState<LayoutOverride>('auto');
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

  const autoOrientation = useMemo(
    () => determineLayoutOrientation(windowDims.width, windowDims.height),
    [windowDims.width, windowDims.height]
  );

  const layoutOrientation: LayoutOrientation =
    layoutOverride === 'auto' ? autoOrientation : layoutOverride;

  const boardSize = useMemo(
    () =>
      calculateOptimalBoardSize(
        windowDims.width,
        windowDims.height,
        layoutOrientation
      ),
    [windowDims.width, windowDims.height, layoutOrientation]
  );

  return {
    layoutOverride,
    setLayoutOverride,
    layoutOrientation,
    boardSize,
  };
}
