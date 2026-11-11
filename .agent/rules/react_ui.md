---
trigger: always_on
description: Các quy tắc bắt buộc (Mandatory Rules) về thiết kế giao diện React 18, TypeScript 5, Tailwind CSS v4, bố cục thích ứng, bàn cờ Canvas 1:1 và trải nghiệm người dùng (UI/UX).
---

# React UI & Frontend Mandatory Rules (Quy tắc Bắt buộc về Giao diện React)

## 1. Visual Identity & Theme (Bản sắc Thị giác & Chủ đề)
- **Dark Rosewood & Lacquerware Aesthetic (Gỗ trầm & Sơn mài cổ điển)**:
  - Luôn duy trì bảng màu chủ đạo của dự án: nền chính `#14110F`, bề mặt thẻ/panel `#181412` / `#1C1815` / `#221D19`, chữ sáng `#F5EFE6`, viền `border-stone-800` / `border-amber-500/40`.
  - Màu nhấn ngữ nghĩa: Đỏ son (`red-600` / `red-400`) cho Phe Đỏ & Chiếu bí; Vàng hổ phách (`amber-500` / `amber-300`) cho Phe Đen, trạng thái chọn, cảnh báo Chiếu tướng và Preset; Xanh ngọc bích (`emerald-400`) cho nước đi hợp lệ và ưu thế điểm (`+điểm`).
- **Typography (Kiểu chữ)**:
  - Dùng `.font-display-serif` cho chữ Hán trên quân cờ (`帥`, `將`, `車`, `馬`, `炮`, ...) và tiêu đề trang trọng.
  - Dùng `.font-mono-tabular` (`tabular-nums`) cho mọi số liệu biến thiên theo thời gian thực (đồng hồ đếm giây, số nút `nodes`, độ sâu `depth`, điểm lượng giá, số lượng quân `×2`, ưu thế `+điểm`) để tránh giật khung hình (layout shift).

## 2. Board Canvas & Responsive Layout (Bàn cờ Canvas & Bố cục Thích ứng)
- **Strict 1:1 Square Board**:
  - Bàn cờ `BoardCanvas.tsx` bắt buộc giữ tỉ lệ vuông **1:1** (`BOARD_SIZE = 554.0`), được tính toán kích thước động qua `calculateOptimalBoardSize(width, height, orientation)` và nhân với `window.devicePixelRatio` để sắc nét trên màn hình Retina/HiDPI.
  - Tự động lật góc nhìn 180° khi `playerSide === 'black'` để quân của người chơi luôn nằm ở nửa dưới bàn cờ.
- **Dual Orientation Support (`sides` & `stacked`)**:
  - Mọi thay đổi trên `App.tsx` và `CapturedPiecesTray.tsx` bắt buộc phải hỗ trợ hoàn hảo cả 2 chế độ bố cục:
    1. **`sides` (Hai bên - Màn hình ngang)**: Khay đối thủ bên trái | Bàn cờ ở giữa | Khay người chơi bên phải (chiều cao khay khớp với `boardSize`).
    2. **`stacked` (Trên dưới - Màn hình dọc/Mobile)**: Khay đối thủ phía trên | Bàn cờ ở giữa | Khay người chơi phía dưới (chiều rộng khay khớp với `boardSize`).
- **No Native Browser Dialogs**:
  - **CẤM** sử dụng `window.alert()`, `window.confirm()`, hoặc `window.prompt()` (do ứng dụng chạy trong môi trường iFrame / PWA). Luôn sử dụng inline feedback banner hoặc custom Modal của React.

## 3. Interactive UX & Puzzle Mode (Tương tác & Chế độ Giải Cờ Thế)
- **Non-blocking AI & Telemetry HUD**:
  - Khi AI đang suy nghĩ (`isThinking === true`), phải khóa thao tác đặt/đi quân trên bàn cờ (`disabled`), hiển thị `ThinkingIndicator` ngay trung tâm bàn cờ, và vô hiệu hóa các nút chuyển chế độ / hoàn tác để tránh xung đột trạng thái.
- **Puzzle Setup Ergonomics (`gameMode === 'Puzzle'` & `puzzleStage === 'setup'`)**:
  - Hỗ trợ đồng thời cả **Kéo-thả (Drag & Drop)** và **Nhấp chọn (Click-to-place / Click-to-move)** quân cờ từ `CapturedPiecesTray` vào `BoardCanvas` hoặc di chuyển giữa các ô trên bàn cờ.
  - Hỗ trợ **Click chuột phải (`onContextMenu`)** lên ô cờ để xóa nhanh quân cờ về lại khay.
  - Hiển thị con trỏ chuột hình quân cờ gỗ (`getPieceCursorStyle`) khi người chơi đang chọn một quân cờ.
  - Không cho phép bắt đầu giải thế cờ nếu `validateBoardSetup(board)` trả về `valid: false` (ví dụ: thiếu Tướng, Tướng ngoài cung, lộ mặt tướng, quá số lượng quân chuẩn).

## 4. Component Architecture & TypeScript Standards
- **Functional Components & Hooks**:
  - Chỉ sử dụng React Functional Components, quản lý bất đồng bộ và sự kiện bằng `useState`, `useEffect`, `useRef`, `useCallback`, `useMemo`.
  - Sử dụng Tailwind CSS v4 (qua `@import "tailwindcss";` trong `src/App.css`); chỉ dùng `style={{ ... }}` cho các giá trị động tính toán theo pixel (`maxWidth`, `height`, `width` theo `boardSize` hoặc `cursor` động).
- **Strict Typing & Verification**:
  - Không dùng `any`; mọi props và state phải định nghĩa rõ trong `src/types/game.ts` hoặc interface của component.
  - Sau mỗi lần chỉnh sửa UI, bắt buộc chạy `npm test`, `npm run lint` và `npm run build` để đảm bảo không phát sinh lỗi kiểu hoặc lỗi đóng gói.
- **Architecture & Skill Synchronization (Đồng bộ Kiến trúc & Agent Skill)**:
  - Khi thêm/xóa/tái cấu trúc Component, thay đổi luồng tương tác UI ↔ Web Worker Pool (`AiPoolManager`), hoặc thay đổi giao diện cấu hình `ConfigModal.tsx`, **BẮT BUỘC** cập nhật đồng bộ `ARCHITECTURE.md` và các Agent Skill liên quan (`.agent/skills/react_ui_design/SKILL.md`, `.agent/skills/wasm_client_ui/SKILL.md`).
