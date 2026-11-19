import type { Piece, PieceColor, Square } from '../../types/game.js';
import {
  BOARD_WIDTH,
  BOARD_HEIGHT,
  PADDING,
  CELL_WIDTH,
  CELL_HEIGHT,
  FIDUCIAL_POINTS,
  getPieceSymbol,
  getVisualCoords,
} from './boardCoords.js';
import {
  type CameraParams,
  WOOD_MIN,
  WOOD_MAX,
  PIECE_RADIUS,
  getVisualRow,
  projectBoardPoint,
} from './boardProjection.js';

export interface DrawPiece3DOptions {
  isSelected: boolean;
  liftZ?: number;
  alpha?: number;
  isCaptureTarget?: boolean;
}

export interface HoverAndSetupHighlightParams {
  hoverTarget: [number, number] | null;
  canHighlightHover: boolean;
  isSetupMode: boolean;
  selectedEmptySquare: [number, number] | null;
  playerSide: PieceColor;
}

function getCosTilt(cam: CameraParams): number {
  return Math.max(0.25, Math.cos((cam.tiltDeg * Math.PI) / 180));
}

function traceProjectedQuad(
  ctx: CanvasRenderingContext2D,
  points: readonly { x: number; y: number }[]
): void {
  const first = points[0];
  if (!first) return;
  ctx.beginPath();
  ctx.moveTo(first.x, first.y);
  for (let i = 1; i < points.length; i++) {
    const pt = points[i];
    if (pt) ctx.lineTo(pt.x, pt.y);
  }
  ctx.closePath();
}

function strokeBoardLine(
  ctx: CanvasRenderingContext2D,
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  cam: CameraParams
): void {
  const p1 = projectBoardPoint(x1, y1, 0, cam);
  const p2 = projectBoardPoint(x2, y2, 0, cam);
  ctx.beginPath();
  ctx.moveTo(p1.x, p1.y);
  ctx.lineTo(p2.x, p2.y);
  ctx.stroke();
}

function strokeBoardRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  cam: CameraParams
): void {
  const pts = [
    projectBoardPoint(x, y, 0, cam),
    projectBoardPoint(x + w, y, 0, cam),
    projectBoardPoint(x + w, y + h, 0, cam),
    projectBoardPoint(x, y + h, 0, cam),
  ];
  traceProjectedQuad(ctx, pts);
  ctx.stroke();
}

function fillBoardRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  cam: CameraParams
): void {
  const pts = [
    projectBoardPoint(x, y, 0, cam),
    projectBoardPoint(x + w, y, 0, cam),
    projectBoardPoint(x + w, y + h, 0, cam),
    projectBoardPoint(x, y + h, 0, cam),
  ];
  traceProjectedQuad(ctx, pts);
  ctx.fill();
}

function drawFiducialMark3D(
  ctx: CanvasRenderingContext2D,
  bx: number,
  by: number,
  visCol: number,
  cam: CameraParams
): void {
  const offset = 4.5;
  const len = 9;
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.35;

  const drawL = (dxSign: number, dySign: number) => {
    const p1 = projectBoardPoint(bx + dxSign * (offset + len), by + dySign * offset, 0, cam);
    const p2 = projectBoardPoint(bx + dxSign * offset, by + dySign * offset, 0, cam);
    const p3 = projectBoardPoint(bx + dxSign * offset, by + dySign * (offset + len), 0, cam);
    ctx.beginPath();
    ctx.moveTo(p1.x, p1.y);
    ctx.lineTo(p2.x, p2.y);
    ctx.lineTo(p3.x, p3.y);
    ctx.stroke();
  };

  if (visCol > 0) {
    drawL(-1, -1);
    drawL(-1, 1);
  }
  if (visCol < 8) {
    drawL(1, -1);
    drawL(1, 1);
  }
}

/**
 * Sorts occupied board squares from far-to-near (Painter's Algorithm) so closer pieces
 * naturally overlap farther ones in 2.5D/3D view.
 */
export function sortSquaresBackToFront(
  board: Square[],
  playerSide: PieceColor
): Square[] {
  return board
    .filter((sq) => sq.piece !== null)
    .sort((a, b) => {
      const rowDiff = getVisualRow(a.row, playerSide) - getVisualRow(b.row, playerSide);
      if (rowDiff !== 0) return rowDiff;
      const aVisCol = playerSide === 'black' ? 8 - a.col : a.col;
      const bVisCol = playerSide === 'black' ? 8 - b.col : b.col;
      return Math.abs(bVisCol - 4) - Math.abs(aVisCol - 4);
    });
}

/**
 * Renders the dark lacquerware tabletop backdrop and multi-layer slab drop shadow.
 */
export function drawTabletopBackground(
  ctx: CanvasRenderingContext2D,
  cam: CameraParams
): void {
  ctx.save();
  const bgGrad = ctx.createRadialGradient(
    BOARD_WIDTH * 0.5,
    BOARD_HEIGHT * 0.42,
    40,
    BOARD_WIDTH * 0.5,
    BOARD_HEIGHT * 0.5,
    BOARD_WIDTH * 0.78
  );
  bgGrad.addColorStop(0, '#271C16');
  bgGrad.addColorStop(0.55, '#19130F');
  bgGrad.addColorStop(1, '#120D0A');
  ctx.fillStyle = bgGrad;
  ctx.fillRect(0, 0, BOARD_WIDTH, BOARD_HEIGHT);

  if (cam.slabThickness > 0.5) {
    const zBase = -cam.slabThickness;
    const baseQuad = [
      projectBoardPoint(WOOD_MIN, WOOD_MIN, zBase, cam),
      projectBoardPoint(WOOD_MAX, WOOD_MIN, zBase, cam),
      projectBoardPoint(WOOD_MAX, WOOD_MAX, zBase, cam),
      projectBoardPoint(WOOD_MIN, WOOD_MAX, zBase, cam),
    ];

    // Outer ambient shadow underneath the raised wooden slab
    const spread = Math.max(0.2, cam.shadowSpread);
    ctx.save();
    ctx.shadowColor = 'rgba(0, 0, 0, 0.72)';
    ctx.shadowBlur = 22 * spread;
    ctx.shadowOffsetX = 3 * spread;
    ctx.shadowOffsetY = 10 * spread;
    traceProjectedQuad(ctx, baseQuad);
    ctx.fillStyle = 'rgba(8, 5, 3, 0.65)';
    ctx.fill();
    ctx.restore();
  }
  ctx.restore();
}

/**
 * Renders the 3D beveled wooden board slab, inlaid gold edge trim, wood grain,
 * grid lines, palaces, fiducials, and foreshortened river calligraphy.
 */
export function drawBoardSlab3D(
  ctx: CanvasRenderingContext2D,
  playerSide: PieceColor,
  isSetupMode: boolean,
  cam: CameraParams
): void {
  const cosT = getCosTilt(cam);
  const tl = projectBoardPoint(WOOD_MIN, WOOD_MIN, 0, cam);
  const tr = projectBoardPoint(WOOD_MAX, WOOD_MIN, 0, cam);
  const br = projectBoardPoint(WOOD_MAX, WOOD_MAX, 0, cam);
  const bl = projectBoardPoint(WOOD_MIN, WOOD_MAX, 0, cam);

  if (cam.slabThickness > 0.5) {
    const zBot = -cam.slabThickness;
    const blB = projectBoardPoint(WOOD_MIN, WOOD_MAX, zBot, cam);
    const brB = projectBoardPoint(WOOD_MAX, WOOD_MAX, zBot, cam);

    // Front thick rosewood slab wall
    ctx.save();
    traceProjectedQuad(ctx, [bl, br, brB, blB]);
    const frontGrad = ctx.createLinearGradient(0, bl.y, 0, blB.y);
    frontGrad.addColorStop(0, '#6B3F22');
    frontGrad.addColorStop(0.25, '#4F2C17');
    frontGrad.addColorStop(0.75, '#351C0E');
    frontGrad.addColorStop(1, '#201007');
    ctx.fillStyle = frontGrad;
    ctx.fill();

    // Top bevel highlight strip along the front edge
    ctx.strokeStyle = 'rgba(236, 200, 148, 0.52)';
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(bl.x + 1, bl.y);
    ctx.lineTo(br.x - 1, br.y);
    ctx.stroke();

    // Antique brass inlay line across the front wooden thickness
    const midLeftX = bl.x + (blB.x - bl.x) * 0.46;
    const midLeftY = bl.y + (blB.y - bl.y) * 0.46;
    const midRightX = br.x + (brB.x - br.x) * 0.46;
    const midRightY = br.y + (brB.y - br.y) * 0.46;
    ctx.strokeStyle = 'rgba(217, 169, 98, 0.34)';
    ctx.lineWidth = 1.0;
    ctx.beginPath();
    ctx.moveTo(midLeftX + 6, midLeftY);
    ctx.lineTo(midRightX - 6, midRightY);
    ctx.stroke();

    // Subtle dark bottom border of front wall
    ctx.strokeStyle = 'rgba(10, 5, 2, 0.85)';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(blB.x, blB.y);
    ctx.lineTo(brB.x, brB.y);
    ctx.stroke();
    ctx.restore();
  }

  // Top playing surface clipped to projected wooden face
  ctx.save();
  traceProjectedQuad(ctx, [tl, tr, br, bl]);
  ctx.clip();

  const surfaceGrad = ctx.createLinearGradient(0, tl.y, 0, bl.y);
  surfaceGrad.addColorStop(0, '#F1DBB9');
  surfaceGrad.addColorStop(0.5, '#E4C495');
  surfaceGrad.addColorStop(1, '#D3AD78');
  ctx.fillStyle = surfaceGrad;
  ctx.fillRect(0, 0, BOARD_WIDTH, BOARD_HEIGHT);

  // Subtle horizontal wood grain streaks
  ctx.strokeStyle = 'rgba(110, 68, 34, 0.055)';
  ctx.lineWidth = 1.0;
  for (let i = 0; i < 18; i++) {
    const gy = WOOD_MIN + ((WOOD_MAX - WOOD_MIN) * (i + 0.5)) / 18;
    const wobble = Math.sin(i * 1.9) * 3.5;
    const g1 = projectBoardPoint(WOOD_MIN, gy, 0, cam);
    const g2 = projectBoardPoint(WOOD_MAX, gy + wobble, 0, cam);
    ctx.beginPath();
    ctx.moveTo(g1.x, g1.y);
    ctx.lineTo(g2.x, g2.y);
    ctx.stroke();
  }

  // Outer perimeter bevel shadow
  traceProjectedQuad(ctx, [tl, tr, br, bl]);
  ctx.strokeStyle = 'rgba(74, 44, 22, 0.65)';
  ctx.lineWidth = 2.2;
  ctx.stroke();

  // Outer heavy frame
  ctx.strokeStyle = '#4A2C16';
  ctx.lineWidth = 3.2;
  strokeBoardRect(ctx, PADDING - 8, PADDING - 8, 8 * CELL_WIDTH + 16, 9 * CELL_HEIGHT + 16, cam);

  // Inner grid frame
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.5;
  strokeBoardRect(ctx, PADDING, PADDING, 8 * CELL_WIDTH, 9 * CELL_HEIGHT, cam);

  // Subtle river band shading
  ctx.fillStyle = 'rgba(107, 66, 38, 0.07)';
  fillBoardRect(
    ctx,
    PADDING + 1,
    PADDING + 4 * CELL_HEIGHT + 1,
    8 * CELL_WIDTH - 2,
    CELL_HEIGHT - 2,
    cam
  );

  // 10 horizontal lines
  ctx.strokeStyle = '#6B4226';
  ctx.lineWidth = 1.45;
  for (let r = 0; r < 10; r++) {
    const y = r * CELL_HEIGHT + PADDING;
    strokeBoardLine(ctx, PADDING, y, PADDING + 8 * CELL_WIDTH, y, cam);
  }

  // 9 vertical lines (broken across the river for inner columns)
  for (let c = 0; c < 9; c++) {
    const x = c * CELL_WIDTH + PADDING;
    if (c === 0 || c === 8) {
      strokeBoardLine(ctx, x, PADDING, x, PADDING + 9 * CELL_HEIGHT, cam);
    } else {
      strokeBoardLine(ctx, x, PADDING, x, PADDING + 4 * CELL_HEIGHT, cam);
      strokeBoardLine(ctx, x, PADDING + 5 * CELL_HEIGHT, x, PADDING + 9 * CELL_HEIGHT, cam);
    }
  }

  // Palace diagonals
  const drawPalaceDiagonals = (startVisRow: number) => {
    const x1 = PADDING + 3 * CELL_WIDTH;
    const x2 = PADDING + 5 * CELL_WIDTH;
    const y1 = PADDING + startVisRow * CELL_HEIGHT;
    const y2 = PADDING + (startVisRow + 2) * CELL_HEIGHT;
    ctx.strokeStyle = '#6B4226';
    ctx.lineWidth = 1.35;
    strokeBoardLine(ctx, x1, y1, x2, y2, cam);
    strokeBoardLine(ctx, x2, y1, x1, y2, cam);
  };
  drawPalaceDiagonals(0);
  drawPalaceDiagonals(7);

  // Setup mode palace zone shading
  if (isSetupMode) {
    ctx.save();
    ctx.fillStyle = 'rgba(217, 119, 6, 0.09)';
    ctx.strokeStyle = 'rgba(217, 119, 6, 0.38)';
    ctx.lineWidth = 1.2;
    ctx.setLineDash([4, 4]);
    fillBoardRect(ctx, PADDING + 3 * CELL_WIDTH, PADDING, 2 * CELL_WIDTH, 2 * CELL_HEIGHT, cam);
    strokeBoardRect(ctx, PADDING + 3 * CELL_WIDTH, PADDING, 2 * CELL_WIDTH, 2 * CELL_HEIGHT, cam);
    fillBoardRect(
      ctx,
      PADDING + 3 * CELL_WIDTH,
      PADDING + 7 * CELL_HEIGHT,
      2 * CELL_WIDTH,
      2 * CELL_HEIGHT,
      cam
    );
    strokeBoardRect(
      ctx,
      PADDING + 3 * CELL_WIDTH,
      PADDING + 7 * CELL_HEIGHT,
      2 * CELL_WIDTH,
      2 * CELL_HEIGHT,
      cam
    );
    ctx.restore();
  }

  // Fiducial markers for Cannons and Soldiers
  FIDUCIAL_POINTS.forEach(([r, c]) => {
    const { x, y } = getVisualCoords(r, c, playerSide);
    const visCol = playerSide === 'black' ? 8 - c : c;
    drawFiducialMark3D(ctx, x, y, visCol, cam);
  });

  // Perspective-foreshortened river calligraphy
  const riverY = PADDING + 4.5 * CELL_HEIGHT;
  const riverLabels: [string, number][] =
    playerSide === 'red'
      ? [
          ['楚 河', PADDING + 2 * CELL_WIDTH],
          ['漢 界', PADDING + 6 * CELL_WIDTH],
        ]
      : [
          ['漢 界', PADDING + 2 * CELL_WIDTH],
          ['楚 河', PADDING + 6 * CELL_WIDTH],
        ];

  for (const [label, bx] of riverLabels) {
    const pt = projectBoardPoint(bx, riverY, 0, cam);
    ctx.save();
    ctx.translate(pt.x, pt.y);
    ctx.scale(pt.scale, pt.scale * cosT);
    ctx.font = 'bold 22px "Noto Serif", "KaiTi", "SimSun", "STKaiti", serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = 'rgba(255, 245, 225, 0.38)';
    ctx.fillText(label, 0, 1.2);
    ctx.fillStyle = '#5C381E';
    ctx.fillText(label, 0, 0);
    ctx.restore();
  }

  ctx.restore();
}

/**
 * Draws the origin (dashed amber) and destination (solid jade) highlight quads on z = 0.
 */
export function drawLastMoveBox3D(
  ctx: CanvasRenderingContext2D,
  lastMove: { from_row: number; from_col: number; to_row: number; to_col: number },
  playerSide: PieceColor,
  cam: CameraParams
): void {
  const fromCoord = getVisualCoords(lastMove.from_row, lastMove.from_col, playerSide);
  const toCoord = getVisualCoords(lastMove.to_row, lastMove.to_col, playerSide);

  ctx.save();
  ctx.strokeStyle = 'rgba(180, 83, 9, 0.8)';
  ctx.lineWidth = 2.4;
  ctx.setLineDash([5, 4]);
  strokeBoardRect(ctx, fromCoord.x - 24, fromCoord.y - 24, 48, 48, cam);

  ctx.setLineDash([]);
  ctx.strokeStyle = 'rgba(21, 128, 61, 0.92)';
  ctx.lineWidth = 2.8;
  strokeBoardRect(ctx, toCoord.x - 25, toCoord.y - 25, 50, 50, cam);
  ctx.restore();
}

/**
 * Draws legal destination discs and capture rings on the z = 0 board surface.
 */
export function drawLegalMoveMarkers3D(
  ctx: CanvasRenderingContext2D,
  legalMoves: [number, number][],
  board: Square[],
  playerSide: PieceColor,
  cam: CameraParams
): void {
  const cosT = getCosTilt(cam);
  legalMoves.forEach(([r, c]) => {
    const { x: bx, y: by } = getVisualCoords(r, c, playerSide);
    const targetSquare = board.find((s) => s.row === r && s.col === c);
    const isCapture = Boolean(targetSquare && targetSquare.piece);
    const pt = projectBoardPoint(bx, by, 0, cam);

    ctx.save();
    if (isCapture) {
      const rx = 26 * pt.scale;
      const ry = Math.max(4, rx * cosT);
      ctx.strokeStyle = 'rgba(220, 38, 38, 0.95)';
      ctx.lineWidth = 3.1 * pt.scale;
      ctx.beginPath();
      ctx.ellipse(pt.x, pt.y, rx, ry, 0, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      const rx = 8.2 * pt.scale;
      const ry = Math.max(3, rx * cosT);
      ctx.fillStyle = 'rgba(22, 163, 74, 0.92)';
      ctx.beginPath();
      ctx.ellipse(pt.x, pt.y, rx, ry, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = '#DCFCE7';
      ctx.lineWidth = 1.5 * pt.scale;
      ctx.stroke();
    }
    ctx.restore();
  });
}

/**
 * Draws projected hover-target and empty-square setup selection rings on z = 0.
 */
export function drawHoverAndSetupHighlights3D(
  ctx: CanvasRenderingContext2D,
  params: HoverAndSetupHighlightParams,
  cam: CameraParams
): void {
  const cosT = getCosTilt(cam);

  if (params.hoverTarget && params.canHighlightHover) {
    const { x: bx, y: by } = getVisualCoords(
      params.hoverTarget[0],
      params.hoverTarget[1],
      params.playerSide
    );
    const pt = projectBoardPoint(bx, by, 0, cam);
    const rx = 26 * pt.scale;
    const ry = Math.max(4, rx * cosT);
    ctx.save();
    ctx.fillStyle = 'rgba(245, 158, 11, 0.24)';
    ctx.strokeStyle = '#D97706';
    ctx.lineWidth = 2.6 * pt.scale;
    ctx.beginPath();
    ctx.ellipse(pt.x, pt.y, rx, ry, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  if (params.isSetupMode && params.selectedEmptySquare) {
    const { x: bx, y: by } = getVisualCoords(
      params.selectedEmptySquare[0],
      params.selectedEmptySquare[1],
      params.playerSide
    );
    const pt = projectBoardPoint(bx, by, 0, cam);
    const rx = 22 * pt.scale;
    const ry = Math.max(4, rx * cosT);
    ctx.save();
    ctx.strokeStyle = '#F59E0B';
    ctx.lineWidth = 2.2 * pt.scale;
    ctx.setLineDash([4, 3]);
    ctx.beginPath();
    ctx.ellipse(pt.x, pt.y, rx, ry, 0, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }
}

/**
 * Renders a cylindrical turned-wood 3D Xiangqi piece with ground cast shadow,
 * depth-scaled sidewall gradient, engraved rim, and foreshortened calligraphy glyph.
 */
export function drawPiece3D(
  ctx: CanvasRenderingContext2D,
  bx: number,
  by: number,
  piece: Piece,
  options: DrawPiece3DOptions,
  cam: CameraParams
): void {
  const liftZ = options.liftZ ?? 0;
  const alpha = options.alpha ?? 1;
  const cosT = getCosTilt(cam);

  const groundPt = projectBoardPoint(bx, by, 0, cam);
  const bottomPt = projectBoardPoint(bx, by, liftZ, cam);
  const topPt = projectBoardPoint(bx, by, liftZ + cam.pieceHeight, cam);

  const rxB = PIECE_RADIUS * bottomPt.scale;
  const ryB = Math.max(4, rxB * cosT);
  const rxT = PIECE_RADIUS * topPt.scale;
  const ryT = Math.max(4, rxT * cosT);

  ctx.save();
  ctx.globalAlpha = alpha;

  // 1. Cast shadow on z = 0 board surface
  const shadowLiftFactor = (cam.pieceHeight * 0.25 + liftZ * 0.48) * Math.max(0.25, cam.shadowSpread);
  const shadowX = groundPt.x + shadowLiftFactor * 0.55 * groundPt.scale;
  const shadowY = groundPt.y + shadowLiftFactor * 0.42 * groundPt.scale;
  const shadowScale = 1 + Math.min(0.25, liftZ * 0.004);
  const shadowOpacity = Math.max(0.16, 0.42 - liftZ * 0.005);

  ctx.save();
  ctx.fillStyle = `rgba(18, 9, 3, ${shadowOpacity.toFixed(3)})`;
  ctx.filter = `blur(${(2.4 + liftZ * 0.14).toFixed(1)}px)`;
  ctx.beginPath();
  ctx.ellipse(
    shadowX,
    shadowY,
    PIECE_RADIUS * groundPt.scale * shadowScale,
    Math.max(4, PIECE_RADIUS * groundPt.scale * cosT * shadowScale),
    0,
    0,
    Math.PI * 2
  );
  ctx.fill();
  ctx.restore();

  // Tight ambient occlusion ring at piece base when resting on the board
  if (liftZ < 1.5) {
    ctx.save();
    ctx.fillStyle = 'rgba(20, 10, 4, 0.34)';
    ctx.beginPath();
    ctx.ellipse(groundPt.x, groundPt.y + 1, rxB * 1.02, ryB * 1.02, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  }

  // 2. Selection golden aura on board surface
  if (options.isSelected) {
    ctx.save();
    ctx.fillStyle = 'rgba(245, 158, 11, 0.36)';
    ctx.beginPath();
    ctx.ellipse(groundPt.x, groundPt.y, rxB * 1.18, ryB * 1.18, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  }

  // 3. Cylindrical turned-wood sidewall connecting bottom and top ellipses
  if (cam.pieceHeight > 0.3) {
    const sideGrad = ctx.createLinearGradient(
      bottomPt.x - rxB,
      0,
      bottomPt.x + rxB,
      0
    );
    if (piece.color === 'red') {
      sideGrad.addColorStop(0, '#784A26');
      sideGrad.addColorStop(0.24, '#C89D6C');
      sideGrad.addColorStop(0.45, '#DFBC8C');
      sideGrad.addColorStop(0.76, '#A67546');
      sideGrad.addColorStop(1, '#5D351A');
    } else {
      sideGrad.addColorStop(0, '#5E4932');
      sideGrad.addColorStop(0.24, '#B59D7D');
      sideGrad.addColorStop(0.45, '#CBB697');
      sideGrad.addColorStop(0.76, '#8F775A');
      sideGrad.addColorStop(1, '#473623');
    }

    ctx.beginPath();
    ctx.moveTo(topPt.x - rxT, topPt.y);
    ctx.lineTo(bottomPt.x - rxB, bottomPt.y);
    ctx.ellipse(bottomPt.x, bottomPt.y, rxB, ryB, 0, Math.PI, 0, true);
    ctx.lineTo(topPt.x + rxT, topPt.y);
    ctx.ellipse(topPt.x, topPt.y, rxT, ryT, 0, 0, Math.PI, false);
    ctx.closePath();
    ctx.fillStyle = sideGrad;
    ctx.fill();

    ctx.strokeStyle = piece.color === 'red' ? '#4A2811' : '#2C2217';
    ctx.lineWidth = 1.1 * bottomPt.scale;
    ctx.beginPath();
    ctx.ellipse(bottomPt.x, bottomPt.y, rxB, ryB, 0, 0, Math.PI, false);
    ctx.stroke();
  }

  // 4. Ivory-wood top face
  const topGrad = ctx.createRadialGradient(
    topPt.x - rxT * 0.28,
    topPt.y - ryT * 0.28,
    2 * topPt.scale,
    topPt.x,
    topPt.y,
    rxT
  );
  if (piece.color === 'red') {
    topGrad.addColorStop(0, '#FFFDF9');
    topGrad.addColorStop(0.72, '#FBF0DC');
    topGrad.addColorStop(1, '#E6C99E');
  } else {
    topGrad.addColorStop(0, '#FAF4E8');
    topGrad.addColorStop(0.72, '#EEE0C8');
    topGrad.addColorStop(1, '#D5BC93');
  }

  ctx.beginPath();
  ctx.ellipse(topPt.x, topPt.y, rxT, ryT, 0, 0, Math.PI * 2);
  ctx.fillStyle = topGrad;
  ctx.fill();

  // Outer carved rim & inner engraved ring
  ctx.lineWidth = 2.2 * topPt.scale;
  ctx.strokeStyle = piece.color === 'red' ? '#B91C1C' : '#1E293B';
  ctx.stroke();

  const innerRx = Math.max(2, rxT - 3.8 * topPt.scale);
  const innerRy = Math.max(2, ryT - 3.8 * topPt.scale * cosT);
  ctx.beginPath();
  ctx.ellipse(topPt.x, topPt.y, innerRx, innerRy, 0, 0, Math.PI * 2);
  ctx.lineWidth = 1.1 * topPt.scale;
  ctx.strokeStyle = piece.color === 'red' ? '#DC2626' : '#475569';
  ctx.stroke();

  // Selection golden halo around top disc
  if (options.isSelected) {
    ctx.beginPath();
    ctx.ellipse(
      topPt.x,
      topPt.y,
      rxT + 2.2 * topPt.scale,
      ryT + 2.2 * topPt.scale * cosT,
      0,
      0,
      Math.PI * 2
    );
    ctx.lineWidth = 2.5 * topPt.scale;
    ctx.strokeStyle = '#F59E0B';
    ctx.stroke();
  }

  // Vermilion capture target ring around top disc
  if (options.isCaptureTarget) {
    ctx.beginPath();
    ctx.ellipse(
      topPt.x,
      topPt.y,
      rxT + 2.5 * topPt.scale,
      ryT + 2.5 * topPt.scale * cosT,
      0,
      0,
      Math.PI * 2
    );
    ctx.lineWidth = 2.8 * topPt.scale;
    ctx.strokeStyle = 'rgba(220, 38, 38, 0.95)';
    ctx.stroke();
  }

  // 5. Traditional calligraphy character foreshortened to match top surface plane
  const symbol = getPieceSymbol(piece.type, piece.color);
  ctx.save();
  ctx.translate(topPt.x, topPt.y);
  ctx.scale(topPt.scale, topPt.scale * cosT);
  ctx.font = 'bold 23px "Noto Serif", "KaiTi", "SimSun", "STKaiti", serif';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillStyle = 'rgba(255, 252, 244, 0.65)';
  ctx.fillText(symbol, 0, 1.8);
  ctx.fillStyle = piece.color === 'red' ? '#B91C1C' : '#0F172A';
  ctx.fillText(symbol, 0, 0.8);
  ctx.restore();

  ctx.restore();
}
