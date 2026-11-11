---
name: react_ui_design
description: Guidelines for designing, building, and modifying the React 18 + TypeScript + Tailwind CSS v4 user interface – Dark Lacquerware theme, 1:1 HTML5 BoardCanvas, adaptive CapturedPiecesTray (sides/stacked), Puzzle drag-and-drop UX, and modals/drawers.
---

# React UI & Design System Skill

## Scope

This skill covers all visual, layout, and interactive frontend components in `src/`:
- `src/App.tsx` – Main layout orchestrator, header bar, layout switcher (`auto` / `sides` / `stacked`), cursor binding, and victory/stalemate overlays
- `src/App.css` – Tailwind CSS v4 setup, custom font utilities (`.font-display-serif`, `.font-mono-tabular`), scrollbars, and keyframe animations
- `src/components/BoardCanvas.tsx` – 1:1 HTML5 Canvas Xiangqi board renderer, HiDPI scaling, perspective flipping, custom wooden piece cursor generator (`getPieceCursorStyle`), and drag-and-drop / right-click handlers
- `src/components/CapturedPiecesTray.tsx` – Responsive player card & captured/off-board piece tray (`sides` vertical sidebar vs. `stacked` horizontal bar)
- `src/components/ControlsArea.tsx` – Top control bar for the 4 game modes, Puzzle setup toolbar, side/difficulty selectors, and FEN modal
- `src/components/PuzzleSetupArea.tsx` – Dedicated Puzzle board editor, piece palette, setup validator feedback, and FEN import/export
- `src/components/ThinkingIndicator.tsx` – Board-centered AI thinking HUD overlay with live elapsed timer and search stats
- `src/components/LogPanel.tsx` – Collapsible bottom move log summary bar and slide-up drawer sheet
- `src/components/ConfigModal.tsx` – Per-side Red/Black `EngineConfig` tuning modal (`tt_size_mb`, `mate_score`, `nnue_weight`, `threads`) with draft state committed on explicit "Áp Dụng & Đóng", 2-side sync, hardware-filtered presets, and JSON payload preview
- `src/utils/capturedPieces.ts` – Layout orientation calculation (`determineLayoutOrientation`, `calculateOptimalBoardSize`) and captured/off-board piece grouping

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

### 1. Orientation Detection & Switcher
- `determineLayoutOrientation(width, height)` in `src/utils/capturedPieces.ts` selects `'sides'` when viewport width ≥ 900px and aspect ratio is landscape (`width / height >= 1.15`), otherwise `'stacked'`.
- Users can manually override the layout via the Header switcher (`Tự động` / `Hai bên` / `Trên dưới`).
- `calculateOptimalBoardSize(width, height, orientation)` computes the largest square pixel size (`[260px .. 680px]`) that fits comfortably within the viewport without vertical scrolling.

### 2. Adaptive `CapturedPiecesTray.tsx`
- **`orientation="sides"`**: Renders a vertical `<aside>` (`w-36 sm:w-44`) matching the exact height of `BoardCanvas` (`style={{ height: `${boardSize}px` }}`), with a 3-column grid of miniature 3D wooden pieces.
- **`orientation="stacked"`**: Renders a compact horizontal `<section>` matching the width of `BoardCanvas` (`style={{ width: `${boardSize}px` }}`), placing player identity on the left, horizontally scrollable miniature pieces in the center, and material advantage on the right.
- **Dual Mode Behavior**:
  - **Standard Gameplay**: Displays enemy pieces captured by that player (`extractCapturedPieces(history)`) and relative material advantage (`+điểm`).
  - **Puzzle Setup (`isSetupMode = true`)**: Displays remaining off-board pieces (`extractOffBoardPieces(board)`) from the standard 32-piece set so the user can drag or click them onto the board.

## HTML5 Canvas Board (`BoardCanvas.tsx`)

1. **Logical vs. Physical Coordinates**:
   - Logical canvas space is `554 × 554` (`PADDING = 34`, `CELL_WIDTH = 60.75`, `CELL_HEIGHT = 54.0`).
   - Physical canvas backing store is scaled by `window.devicePixelRatio` (`ctx.scale(dpr * scaleFactor, dpr * scaleFactor)`).
2. **Perspective Flipping**:
   - Row `0` is Red's back rank; Row `9` is Black's back rank.
   - When `playerSide === 'red'`, `canvasY = PADDING + (9 - row) * CELL_HEIGHT` and `canvasX = PADDING + col * CELL_WIDTH`.
   - When `playerSide === 'black'`, both axes invert (`canvasY = PADDING + row * CELL_HEIGHT`, `canvasX = PADDING + (8 - col) * CELL_WIDTH`) so Black is at the bottom.
3. **Interactive Visual Cues**:
   - **Last Move**: Dashed amber ring at origin square + glowing amber highlight at destination square.
   - **Legal Moves**: Jade dot (`#10B981`) on empty target squares; vermilion corner brackets/ring (`#EF4444`) on capturable enemy pieces.
   - **Puzzle Setup Hover & Drag**: Ghost preview of the piece being dragged or selected (`selectedSetupPiece` / `externalDraggedPiece`), plus dynamic CSS cursor (`getPieceCursorStyle`).

## Common UI Tasks

### Adding a new control or modal
1. Style the container with `bg-[#1C1815]` or `bg-[#1A1613]`, `border border-stone-800`, and `rounded-2xl`.
2. Close modals on backdrop click and provide an explicit close (`✕`) button with `aria-label="Đóng"`.
3. Ensure all buttons specify `type="button"` and `cursor-pointer` (or `cursor-not-allowed` when `disabled`).
4. Verify responsive rendering across both `sides` (desktop landscape) and `stacked` (mobile portrait) layouts.
5. Run `npm test && npm run lint && npm run build` to confirm zero regressions.
