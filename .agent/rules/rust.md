---
trigger: always_on
description: Các quy tắc bắt buộc (Mandatory Rules) về an toàn, xử lý lỗi, tiêu chuẩn ngôn ngữ và kiểm thử cho Rust Core/Trainer (WASM & NNUE) và React TypeScript Frontend.
---

# Mandatory Rules (Quy tắc Bắt buộc)

## 1. Safety & Error Handling (An toàn & Xử lý lỗi)
- **FORBIDDEN `unwrap()` / `expect()` trong Rust**:
  - **CẤM TUYỆT ĐỐI** sử dụng `.unwrap()` hoặc `.expect()` trong mã nguồn chính (`cotuong_core/src/` và `cotuong_trainer/src/`).
  - *Ngoại lệ*: Chỉ được phép dùng trong `#[cfg(test)]`, thư mục `tests/`, hoặc các hằng số khởi tạo tĩnh an toàn tuyệt đối.
  - *Hành động*: Phải xử lý lỗi bằng `match`, `if let`, `?`, `map_err`, hoặc `unwrap_or_else`.
- **No Panics**: Engine Rust và giao diện WASM (`cotuong_core/src/wasm.rs`) không được phép panic với bất kỳ dữ liệu đầu vào nào từ phía JavaScript/người dùng (kể cả chuỗi FEN tùy chỉnh từ chế độ Giải Cờ Thế). Mọi lỗi qua ranh giới WASM phải trả về `Result<_, JsValue>`.
- **TypeScript Strictness**: Không sử dụng `any` bừa bãi; luôn định nghĩa rõ kiểu dữ liệu trong `src/types/game.ts` khi giao tiếp với `WasmGameState`, `WasmEngine` và `EngineConfig`.

## 2. Code Integrity (Tính toàn vẹn)
- **Không xóa Logic**: Không tự ý xóa logic phức tạp trong bộ tìm kiếm Alpha-Beta, mạng lượng giá Dual-Head HalfKP NNUE v3 (`nnue.rs`), bộ huấn luyện (`cotuong_trainer`), hoặc luật cờ (bao gồm phát hiện lặp nước 3 lần) nếu không có yêu cầu refactor rõ ràng.
- **Đồng bộ Ranh giới WASM – Polyfill – TypeScript**:
  - Khi thay đổi DTO hoặc chữ ký hàm trong `cotuong_core/src/wasm.rs` hoặc `EngineConfig` (`cotuong_core/src/engine/config.rs`), bắt buộc phải cập nhật đồng bộ trong:
    - `src/types/game.ts`
    - `src/wasm/enginePolyfill.ts` & `src/wasm/cotuong_core.ts`
    - `src/components/ConfigModal.tsx` (nếu liên quan đến `EngineConfig`)
  - Nếu có môi trường Rust/wasm-bindgen, chạy `npm run build:wasm` để cập nhật gói WASM. Không sửa thủ công các file `.wasm` hoặc bindings sinh tự động từ `wasm-bindgen`.
- **Đồng bộ Tài liệu Kiến trúc & Agent Skills (Documentation & Skill Sync)**:
  - Mỗi khi có thay đổi về **kiến trúc hệ thống, cấu trúc thư mục/module, DTO/API, tham số cấu hình (`EngineConfig`), hoặc luồng xử lý dữ liệu/tìm kiếm AI**, **BẮT BUỘC** phải cập nhật đồng bộ ngay trong cùng tác vụ cho:
    1. `ARCHITECTURE.md` (sơ đồ Mermaid, bảng cấu trúc file, mô tả module và luồng tuần tự).
    2. `README.md` (nếu thay đổi tính năng, cấu trúc thư mục, tham số `EngineConfig`, Preset hoặc lệnh CLI).
    3. Các **Agent Skill** tương ứng trong `.agent/skills/`:
       - Thay đổi luật cờ, bàn cờ `Board`, FEN/Puzzle → `.agent/skills/game_logic_rules/SKILL.md`
       - Thay đổi thuật toán tìm kiếm Alpha-Beta/Lazy SMP, `TranspositionTable`, `EngineConfig`, mạng NNUE hoặc `cotuong_trainer` → `.agent/skills/xiangqi_engine_tuning/SKILL.md`
       - Thay đổi cầu nối `wasm.rs`, `enginePolyfill.ts`, `aiPoolManager.ts`, `ai.worker.ts`, `modelStorage.ts` → `.agent/skills/wasm_client_ui/SKILL.md`
       - Thay đổi giao diện, bố cục `sides`/`stacked`, component React/Tailwind → `.agent/skills/react_ui_design/SKILL.md`
       - Thay đổi quy chuẩn Rust, lints, hoặc quy trình kiểm thử/benchmark → `.agent/skills/rust_development/SKILL.md`

## 3. Language Standards (Tiêu chuẩn Ngôn ngữ)
- **Communication (Trao đổi)**:
  - Luôn sử dụng **Tiếng Việt** để giải thích, thảo luận, báo cáo lỗi và hướng dẫn trong khung chat.
- **Code Comments (Ghi chú trong Code)**:
  - Bắt buộc sử dụng **Tiếng Anh** cho tất cả các comment nằm trong source code (bao gồm Doc comments `///`, JSDoc `/** */` và Inline comments `//`).
  - *Lý do*: Đảm bảo tính chuyên nghiệp và khả năng tương thích quốc tế của mã nguồn.

## 4. Performance & Optimization (Hiệu năng & Tối ưu hóa)
- **No Unnecessary Allocations in Engine Hot Paths**:
  - **CẤM** cấp phát bộ nhớ heap (`Vec::new`, `String::clone`, `Box::new`) bên trong vòng lặp tìm kiếm (`alpha_beta`, `quiescence`, `EngineMoveGen`, `evaluate_dual_forward`).
  - *Hành động*: Sử dụng `MoveList` (mảng tĩnh trên stack `[Move; 128]`), cập nhật tăng dần `NnueAccumulator` và `zobrist_hash` trong `Board::apply_move` / `Board::undo_move`.
- **Non-blocking UI & Multi-threaded Worker Pool**:
  - Mọi tác vụ tìm kiếm nước đi của AI bắt buộc phải chạy bất đồng bộ qua `AiPoolManager` (`src/workers/aiPoolManager.ts`) và các Web Worker (`src/workers/ai.worker.ts`), tuyệt đối không chạy `WasmEngine.search()` trực tiếp trên luồng giao diện chính (main thread).

## 5. Code Style & Idioms (Phong cách & Cấu trúc Code)
- **Rust**:
  - Các hàm nhận chuỗi đầu vào ưu tiên sử dụng `&str` thay vì `String`.
  - Ưu tiên `if let` thay vì `match` cho các trường hợp đơn lẻ; tuân thủ `clippy::indexing_slicing = "deny"` bằng `.get()` / `.get_mut()` hoặc iterator.
  - Hạn chế lồng ghép `if/else` quá sâu (tối đa 2–3 cấp); tách thành các hàm nhỏ có tên rõ nghĩa.
- **React & TypeScript**:
  - Sử dụng Functional Components với React Hooks (`useState`, `useEffect`, `useRef`, `useCallback`, `useMemo`).
  - Sử dụng Tailwind CSS v4 cho giao diện; giữ bàn cờ Canvas luôn vuông tỉ lệ `1:1` và hỗ trợ mượt mà cả hai bố cục `sides` (Hai bên) và `stacked` (Trên - Dưới).

## 6. Quality Assurance (Đảm bảo Chất lượng)
- **Mandatory Checks (Kiểm tra Bắt buộc)**:
  - Sau khi hoàn thành viết code hoặc refactor ở Rust hoặc TypeScript, **BẮT BUỘC** phải kiểm tra:
    1. `cargo fmt --check` (hoặc `cargo fmt`): Định dạng chuẩn Rust.
    2. `cargo clippy --workspace --all-targets -- -D warnings` và `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings`: Kiểm tra lint nghiêm ngặt cho cả native (`cotuong_core`, `cotuong_trainer`) và WASM.
    3. `cargo test --workspace`: Chạy toàn bộ unit tests, mate-solving tests, property-based tests và fuzz tests của Rust.
    4. `npm test`, `npm run lint` & `npm run build`: Chạy unit tests TypeScript, kiểm tra kiểu (`tsc --noEmit`) và đóng gói Vite production.
  - Có thể chạy nhanh quy trình kiểm tra toàn diện bằng script `./test_all.sh`.