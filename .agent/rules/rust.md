---
trigger: always_on
description: Các quy tắc bắt buộc (Mandatory Rules) về an toàn, xử lý lỗi và tiêu chuẩn ngôn ngữ.
---

# Mandatory Rules (Quy tắc Bắt buộc)

## 1. Safety & Error Handling (An toàn & Xử lý lỗi)
- **FORBIDDEN `unwrap()` / `expect()`**: 
  - **CẤM TUYỆT ĐỐI** sử dụng `.unwrap()` hoặc `.expect()` trong mã nguồn chính (production code).
  - *Ngoại lệ*: Chỉ được phép dùng trong `#[test]`, thư mục `tests/`, hoặc các hằng số `static`/`const` an toàn tuyệt đối.
  - *Hành động*: Phải xử lý lỗi bằng `match`, `if let`, `?`, hoặc `unwrap_or_else`.
- **No Panics**: Code không được phép panic với bất kỳ input nào từ người dùng.

## 2. Code Integrity (Tính toàn vẹn)
- **Không xóa Logic**: Không tự ý xóa logic phức tạp hoặc comment quan trọng nếu không có yêu cầu refactor rõ ràng.

## 3. Language Standards (Tiêu chuẩn Ngôn ngữ)
- **Communication (Trao đổi)**: 
  - Luôn sử dụng **Tiếng Việt** để giải thích, thảo luận, báo cáo lỗi và hướng dẫn trong khung chat.
- **Code Comments (Ghi chú trong Code)**: 
  - Bắt buộc sử dụng **Tiếng Anh** cho tất cả các comment nằm trong source code (bao gồm Doc comments `///` và Inline comments `//`).
  - *Lý do*: Đảm bảo tính chuyên nghiệp và khả năng tương thích quốc tế của mã nguồn.

## 4. Performance & Optimization (Hiệu năng & Tối ưu hóa)
- **No `clone()` on Strings**: 
  - **CẤM** gọi `.clone()` trên `String` hoặc `&str` nếu không thực sự cần thiết.
  - *Hành động*: Ưu tiên sử dụng `&str` (string slices) để truyền dữ liệu, tránh cấp phát bộ nhớ không cần thiết.
- **Avoid Unnecessary Allocations**: 
  - Hạn chế tạo mới `String` hoặc `Vec` bên trong các vòng lặp hoặc hàm xử lý dữ liệu lớn.
  - *Thay thế*: Sử dụng `String::with_capacity()` khi biết trước kích thước, hoặc sử dụng các phương thức xử lý slice (`&str`).

## 5. Code Style & Idioms (Phong cách & Cấu trúc Code)
- **Use `&str` for Function Arguments**: 
  - Các hàm nhận chuỗi đầu vào nên sử dụng `&str` thay vì `String` để tăng tính linh hoạt.
  - *Ví dụ*: `fn process(text: &str) -> String` thay vì `fn process(text: String) -> String`.
- **Prefer `if let` over `match` for Single Cases**: 
  - Khi chỉ cần xử lý một trường hợp (ví dụ: `Some(value)`), hãy dùng `if let` thay vì `match` đầy đủ để code ngắn gọn hơn.
- **Avoid Deep Nesting**: 
  - Hạn chế lồng ghép `if/else` quá nhiều cấp (tối đa 2-3 cấp). Nếu logic phức tạp, hãy tách thành các hàm nhỏ hơn.
## 6. Quality Assurance (Đảm bảo Chất lượng)
- **Mandatory Checks (Kiểm tra Bắt buộc)**: 
  - Sau khi hoàn thành viết code hoặc refactor, **BẮT BUỘC** phải chạy bộ ba công cụ sau:
    1. `cargo fmt`: Để tự động định dạng code theo chuẩn chung.
    2. `cargo check`: Để đảm bảo code biên dịch được mà không có lỗi.
    3. `cargo clippy`: Để phát hiện các vấn đề về tối ưu hóa, style và các lỗi tiềm ẩn.
  - *Hành động*: Nếu phát hiện lỗi (error) hoặc cảnh báo (warning), phải sửa ngay lập tức. Không commit code khi còn warning của Clippy (trừ trường hợp false positive đã được đánh dấu `allow`).
- **Continuous Verification (Kiểm tra Liên tục)**:
  - Luôn đảm bảo code sạch sẽ và tuân thủ các quy tắc của Rust trước khi báo cáo hoàn thành task.