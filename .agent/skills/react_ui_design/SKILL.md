---
name: react_ui_design
description: Guidelines for designing, building, and modifying the React 18 + TypeScript + Tailwind CSS v4 user interface – Dark Lacquerware theme, modular hooks/subcomponents, 1:1 HTML5 BoardCanvas, adaptive CapturedPiecesTray (sides/stacked), Puzzle drag-and-drop UX, and modals/drawers.
---

# React UI & Design System Skill

## Scope

This skill covers all visual, layout, and interactive frontend modules in `src/`:
- **Root & Custom Hooks**:
  - `src/App.tsx` – Root layout orchestrator connecting custom hooks (`useResponsiveLayout`, `useXiangqiGame`) with top-level UI regions (`TopHeader`, `ControlsArea`, `BoardPlayArea`, `LogPanel`, `ConfigModal`).
  - `src/hooks/useResponsiveLayout.ts` – Viewport resize listener, `layoutOverride` (`'auto' | 'sides' | 'stacked'`), `layoutOrientation`, and `boardSize` calculation.
  - `src/hooks/useXiangqiGame.ts` – Complete game state, `WasmGameState` + `AiPoolManager` integration, 4 game modes, and Puzzle setup/solving handlers.
  - `src/App.css` – Tailwind CSS v4 setup, custom font utilities (`.font-display-serif`, `.font-mono-tabular`), scrollbars, and keyframe animations.
- **Top-Level Layout & Board Components**:
  - `src/components/TopHeader.tsx` – Brand header, layout switcher (`Tự động` / `Hai bên` / `Trên dưới`), `Chiếu tướng!` alert badge, and active turn indicator.
  - `src/components/BoardPlayArea.tsx` – Unified responsive container rendering `CapturedPiecesTray` + `BoardCanvas` + `ThinkingIndicator` + `GameOverOverlay` in either `'sides'` or `'stacked'` orientation without code duplication.
  - `src/components/GameOverOverlay.tsx` – Checkmate / Stalemate overlay modal rendered inside `BoardCanvas`.
  - `src/components/BoardCanvas.tsx` – 1:1 HTML5 Canvas Xiangqi board component managing mouse/drag-and-drop events and delegating geometry/drawing to `src/components/board/`:
    - `src/components/board/boardCoords.ts` (`& boardCoords.test.ts`) – Pure board constants (`BOARD_SIZE = 554.0`, `PADDING`, `CELL_WIDTH`, `CELL_HEIGHT`, `FIDUCIAL_POINTS`), `getPieceSymbol`, `getVisualCoords`, and `getLogicCoords`.
    - `src/components/board/boardRenderer.ts` – Canvas 2D rendering functions (`drawBoardGridAndRiver`, `drawPiece`, `drawLastMoveBox`, `drawLegalMoveMarkers`, `drawFiducialMark`, and `getPieceCursorStyle`).
- **Captured Pieces Trays**:
  - `src/components/CapturedPiecesTray.tsx` – Responsive player card & captured/off-board piece tray (`sides` vertical sidebar vs. `stacked` horizontal bar).
  - `src/components/tray/MiniWoodenPiece.tsx` – Draggable/clickable 3D miniature wooden piece badge (`MiniWoodenPiece`) and `getTrayRoleLabel` helper.
- **Controls, Puzzle Editor & Modals**:
  - `src/components/ControlsArea.tsx` – Top control bar for the 4 game modes, side/difficulty selectors, and action toolbar; delegates puzzle setup controls to `src/components/controls/`:
    - `src/components/controls/PuzzleSetupToolbar.tsx` – `PuzzleSetupToolbar` and `PuzzleValidationStrip`.
    - `src/components/controls/FenModal.tsx` – Reusable FEN import/export modal shared by `ControlsArea` and `PuzzleSetupArea`.
  - `src/components/PuzzleSetupArea.tsx` – Dedicated Puzzle board editor delegating to `src/components/puzzle/`:
    - `src/components/puzzle/PuzzlePiecePalette.tsx` – Red & Black piece palette and Eraser toggle.
    - `src/components/puzzle/PuzzleMatchSettings.tsx` – First turn, sub-mode, player side, and AI difficulty controls.
  - `src/components/ThinkingIndicator.tsx` – Board-centered AI thinking HUD overlay with live elapsed timer and search stats.
  - `src/components/LogPanel.tsx` – Collapsible bottom move log summary bar and slide-up drawer sheet.
  - `src/components/ConfigModal.tsx` – Per-side Red/Black `EngineConfig` tuning modal delegating to `src/components/config/`:
    - `src/components/config/configPresets.ts` – Hardware-filtered `PRESETS`, `sanitizeImportedConfig`, and `formatRamMb`.
    - `src/components/config/ConfigPresetsSection.tsx` – Quick preset selector cards.
    - `src/components/config/ConfigParamsSection.tsx` – Sliders and quick buttons for `tt_size_mb`, `mate_score`, `nnue_weight`, and `threads`.
    - `src/components/config/ConfigArchitectureSection.tsx` – Dual-Head HalfKP v3 architecture summary and JSON payload preview.
- **Layout & Piece Utilities**:
  - `src/utils/capturedPieces.ts` – Layout orientation calculation (`determineLayoutOrientation`, `calculateOptimalBoardSize`) and captured/off-board piece grouping.

## Design System & Aesthetics

### 1. Color Palette (Dark Rosewood & Lacquerware)
| Role | Hex / Tailwind Token | Usage |
|---|---|---|
| Primary Background | `#14110F` | Main page viewport background (`bg-[#14110F]`) |
| Card / Panel Surface | `#181412`, `#1C1815`, `#221D19` | Control bars, trays, modals, and drawers |
| Primary Text | `#F5EFE6` | Headings, active labels, primary UI text |
| Secondary / Muted Text | `text-stone-400`, `text-stone-500` | Subtitles, metadata, inactive controls |
| Red Side Accent | `red-600`, `red-400`, `#B91C1C` | Red pieces (`帥`), Red turn badges, primary CTA buttons |
| Black Side / Gold Accent | `amber-500`, `amber-300`, `amber-400` | Black side highlights (`將`), selected state rings, check alerts |
| Positive / Legal Accent | `emerald-400`, `emerald-500/20` | Legal move dots, material advantage pills (`+điểm`), valid FEN status |

### 2. Typography Rules
- **Display Serif (`.font-display-serif`)**: Used for traditional Chinese piece glyphs (`帥`, `將`, `仕`, `士`, `相`, `象`, `傌`, `馬`, `俥`, `車`, `炮`, `砲`, `兵`, `卒`), app title, and victory headings.
- **Tabular Monospace (`.font-mono-tabular` / `font-mono`)**: Mandatory for live timers (`0.0s`), search telemetry (`depth`, `nodes`), piece count badges (`×2`), material points (`totalCapturedValue`), and FEN/JSON strings.
- **Sans-Serif (`--font-sans`)**: Clean Vietnamese UI labels, buttons, tooltips, and descriptions.

## Responsive Layout Architecture

### 1. Orientation Detection & Switcher (`useResponsiveLayout.ts` & `TopHeader.tsx`)
- `determineLayoutOrientation(width, height)` in `src/utils/capturedPieces.ts` selects `'sides'` when viewport width ≥ 820px and aspect ratio is landscape (`width / height >= 1.08`), otherwise `'stacked'`.
- Users can manually override the layout via `TopHeader.tsx` (`Tự động` / `Hai bên` / `Trên dưới`).
- `BoardPlayArea.tsx` consumes `layoutOrientation` and `boardSize` to render either the horizontal 3-column layout (`sides`) or vertical stacked layout (`stacked`).

### 2. Adaptive `CapturedPiecesTray.tsx` & `MiniWoodenPiece.tsx`
- **`orientation="sides"`**: Renders a vertical `<aside>` (`w-36 sm:w-44`) matching the exact height of `BoardCanvas` (`style={{ height: `${boardSize}px` }}`), with a 3-column grid of miniature 3D wooden pieces.
- **`orientation="stacked"`**: Renders a compact horizontal `<section>` matching the width of `BoardCanvas` (`style={{ width: `${boardSize}px` }}`), placing player identity on the left, horizontally scrollable miniature pieces in the center, and material advantage on the right.
- **Dual Mode Behavior**:
  - **Standard Gameplay**: Displays enemy pieces captured by that player (`extractCapturedPieces(history)`) and relative material advantage (`+điểm`).
  - **Puzzle Setup (`isSetupMode = true`)**: Displays remaining off-board pieces (`extractOffBoardPieces(board)`) from the standard 32-piece set so the user can drag or click them onto the board.

## HTML5 Canvas Board (`BoardCanvas.tsx`, `boardCoords.ts`, `boardRenderer.ts`)

1. **Logical vs. Physical Coordinates (`boardCoords.ts`)**:
   - Logical canvas space is `554 × 554` (`PADDING = 34`, `CELL_WIDTH = 60.75`, `CELL_HEIGHT = 54.0`).
   - Physical canvas backing store is scaled by `window.devicePixelRatio` (`ctx.scale(dpr * scaleFactor, dpr * scaleFactor)`).
2. **Perspective Flipping (`getVisualCoords` & `getLogicCoords`)**:
   - Row `0` is Red's back rank; Row `9` is Black's back rank.
   - When `playerSide === 'red'`, `canvasY = PADDING + (9 - row) * CELL_HEIGHT` and `canvasX = PADDING + col * CELL_WIDTH`.
   - When `playerSide === 'black'`, both axes invert (`canvasY = PADDING + row * CELL_HEIGHT`, `canvasX = PADDING + (8 - col) * CELL_WIDTH`) so Black is at the bottom.
3. **Interactive Visual Cues (`boardRenderer.ts`)**:
   - **Last Move**: Dashed amber box at origin square + solid green highlight box at destination square (`drawLastMoveBox`).
   - **Legal Moves**: Jade dot (`rgba(22, 163, 74, 0.9)`) on empty target squares; vermilion ring (`#DC2626`) on capturable enemy pieces (`drawLegalMoveMarkers`).
   - **Puzzle Setup Hover & Drag**: Ghost preview of the piece being dragged or selected (`selectedSetupPiece` / `externalDraggedPiece`), plus dynamic CSS cursor (`getPieceCursorStyle`).

## Common UI Tasks

### Adding or modifying a UI component
1. Keep components modular (< 350 lines per file); place domain-specific subcomponents under `src/components/<domain>/` (`board/`, `config/`, `controls/`, `puzzle/`, `tray/`) and state/lifecycle logic in `src/hooks/`.
2. Style containers with `bg-[#1C1815]` or `bg-[#1A1613]`, `border border-stone-800`, and `rounded-2xl`.
3. Close modals on backdrop click and provide an explicit close (`✕`) button with `aria-label="Đóng"`.
4. Ensure all buttons specify `type="button"` and `cursor-pointer` (or `cursor-not-allowed` when `disabled`).
5. Verify responsive rendering across both `sides` (desktop landscape) and `stacked` (mobile portrait) layouts.
6. Run `npm test && npm run lint && npm run build` to confirm zero regressions.
