---
trigger: always_on
description: Các quy tắc bắt buộc (Mandatory Rules) về an toàn, xử lý lỗi, tiêu chuẩn ngôn ngữ và kiểm thử cho Rust Core (WASM) & React TypeScript Frontend.
---

# Mandatory Rules (Quy tắc Bắt buộc)

## 1. Safety & Error Handling (An toàn & Xử lý lỗi)
- **FORBIDDEN `unwrap()` / `expect()` trong Rust**:
  - **CẤM TUYỆT ĐỐI** sử dụng `.unwrap()` hoặc `.expect()` trong mã nguồn chính (`cotuong_core/src/`).
  - *Ngoại lệ*: Chỉ được phép dùng trong `#[cfg(test)]`, thư mục `tests/`, hoặc các hằng số khởi tạo tĩnh an toàn tuyệt đối.
  - *Hành động*: Phải xử lý lỗi bằng `match`, `if let`, `?`, `map_err`, hoặc `unwrap_or_else`.
- **No Panics**: Engine Rust và giao diện WASM (`wasm.rs`) không được phép panic với bất kỳ dữ liệu đầu vào nào từ phía JavaScript/người dùng. Mọi lỗi qua ranh giới WASM phải trả về `Result<_, JsValue>`.
- **TypeScript Strictness**: Không sử dụng `any` bừa bãi; luôn định nghĩa rõ kiểu dữ liệu trong `src/types/game.ts` khi giao tiếp với `WasmGameState` và `WasmEngine`.

## 2. Code Integrity (Tính toàn vẹn)
- **Không xóa Logic**: Không tự ý xóa logic phức tạp trong bộ tìm kiếm Alpha-Beta, hàm lượng giá hoặc luật cờ nếu không có yêu cầu refactor rõ ràng.
- **Đồng bộ Ranh giới WASM – TypeScript**: Khi thay đổi DTO trong `cotuong_core/src/wasm.rs`, bắt buộc cập nhật tương ứng trong `src/types/game.ts` và build lại gói WASM (`npm run build:wasm`).

## 3. Language Standards (Tiêu chuẩn Ngôn ngữ)
- **Communication (Trao đổi)**:
  - Luôn sử dụng **Tiếng Việt** để giải thích, thảo luận, báo cáo lỗi và hướng dẫn trong khung chat.
- **Code Comments (Ghi chú trong Code)**:
  - Bắt buộc sử dụng **Tiếng Anh** cho tất cả các comment nằm trong source code (bao gồm Doc comments `///`, JSDoc `/** */` và Inline comments `//`).
  - *Lý do*: Đảm bảo tính chuyên nghiệp và khả năng tương thích quốc tế của mã nguồn.

## 4. Performance & Optimization (Hiệu năng & Tối ưu hóa)
- **No Unnecessary Allocations in Engine Hot Paths**:
  - **CẤM** cấp phát bộ nhớ heap (`Vec::new`, `String::clone`, `Box::new`) bên trong vòng lặp tìm kiếm (`alpha_beta`, `quiescence`, `EngineMoveGen`).
  - *Hành động*: Sử dụng `MoveList` (mảng tĩnh trên stack `[Move; 128]`), truyền tham chiếu `&str` hoặc slice `&[T]`.
- **Non-blocking UI**:
  - Mọi tác vụ tìm kiếm nước đi của AI bắt buộc phải chạy trên Web Worker (`src/workers/ai.worker.ts`), tuyệt đối không chạy `WasmEngine.search()` trực tiếp trên luồng giao diện chính (main thread).

## 5. Code Style & Idioms (Phong cách & Cấu trúc Code)
- **Rust**:
  - Các hàm nhận chuỗi đầu vào ưu tiên sử dụng `&str` thay vì `String`.
  - Ưu tiên `if let` thay vì `match` cho các trường hợp đơn lẻ.
  - Hạn chế lồng ghép `if/else` quá sâu (tối đa 2–3 cấp); tách thành các hàm nhỏ có tên rõ nghĩa.
- **React & TypeScript**:
  - Sử dụng Functional Components với React Hooks (`useState`, `useEffect`, `useRef`, `useCallback`).
  - Sử dụng Tailwind CSS v4 cho giao diện; giữ bàn cờ Canvas luôn vuông tỉ lệ `1:1` và co giãn linh hoạt theo khung nhìn.

## 6. Quality Assurance (Đảm bảo Chất lượng)
- **Mandatory Checks (Kiểm tra Bắt buộc)**:
  - Sau khi hoàn thành viết code hoặc refactor ở Rust hoặc TypeScript, **BẮT BUỘC** phải kiểm tra:
    1. `cargo fmt --check` (hoặc `cargo fmt`): Định dạng chuẩn Rust.
    2. `cargo clippy --workspace --all-targets -- -D warnings` và `cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings`: Kiểm tra lint nghiêm ngặt cho cả native và WASM.
    3. `cargo test --workspace`: Chạy toàn bộ unit tests và mate-solving tests của Rust.
    4. `npm run lint` & `npm run build`: Kiểm tra kiểu TypeScript (`tsc --noEmit`) và đóng gói Vite production.
  - Có thể chạy nhanh toàn bộ quy trình kiểm tra bằng script `./test_all.sh`.