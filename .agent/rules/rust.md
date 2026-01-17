---
trigger: always_on
glob: "**/*.rs"
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