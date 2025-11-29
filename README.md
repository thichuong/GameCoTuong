# Cờ Tướng (Chinese Chess PWA)

Một ứng dụng Cờ Tướng chạy trên trình duyệt sử dụng Rust và WebAssembly (Leptos).

## Tính năng
- **Chế độ chơi:** Người vs Máy, Máy vs Máy (CvC), Người vs Người.
- **Cấu hình AI:** Tùy chỉnh tham số riêng biệt cho Đỏ và Đen.
- **Giao diện:** Đẹp mắt, hỗ trợ Dark Mode, Responsive (Mobile/Desktop).
- **PWA:** Hỗ trợ cài đặt và chạy offline.

## Cấu Hình AI (Engine Parameters)

Bạn có thể tinh chỉnh sức mạnh và phong cách chơi của máy thông qua bảng cấu hình (Config Panel).

### 1. Giá trị quân cờ (Piece Values)
Điểm số cơ bản cho từng loại quân. AI sẽ ưu tiên bảo vệ quân có giá trị cao và đổi quân giá trị thấp lấy quân giá trị cao.
- **Tốt (Pawn):** Giá trị của quân Tốt (mặc định ~30-50).
- **Sĩ (Advisor):** Giá trị quân Sĩ (mặc định ~120).
- **Tượng (Elephant):** Giá trị quân Tượng (mặc định ~120).
- **Mã (Horse):** Giá trị quân Mã (mặc định ~270).
- **Pháo (Cannon):** Giá trị quân Pháo (mặc định ~285).
- **Xe (Rook):** Giá trị quân Xe (mặc định ~600).
- **Tướng (King):** Giá trị quân Tướng (rất lớn, mặc định ~10000).

### 2. Tham số tìm kiếm (Search Parameters)
Các tham số ảnh hưởng đến thuật toán tìm kiếm Alpha-Beta và các heuristics cắt tỉa.

- **Hash Move (Điểm Hash):**
  - Điểm thưởng cho nước đi tốt nhất được lưu trong bảng băm (Transposition Table) từ lần tìm kiếm trước.
  - Giá trị cao giúp AI ưu tiên đi lại các nước đi tốt đã biết, tăng tốc độ tìm kiếm.

- **Capture Base (Điểm bắt quân):**
  - Điểm thưởng cơ bản cho một nước bắt quân (cộng thêm giá trị quân bị bắt).
  - Giá trị cao làm cho AI hung hãn hơn, ưu tiên xét các nước ăn quân trước (MVV-LVA).

- **Killer Move (Điểm Killer):**
  - Điểm thưởng cho "nước đi sát thủ" (Killer Move) - là nước đi không ăn quân nhưng gây ra cắt tỉa (beta cutoff) ở cùng độ sâu tìm kiếm.
  - Giúp AI nhanh chóng nhận ra các nước đi chiến lược mạnh mà không cần tính toán lại nhiều lần.

- **History Max (Điểm Lịch sử tối đa):**
  - Giới hạn điểm thưởng tối đa cho History Heuristic (thống kê các nước đi tốt theo lịch sử).
  - Giúp AI ưu tiên các nước đi thường xuyên thành công trong quá khứ.

- **Pruning Ratio (Tỉ lệ cắt tỉa):**
  - **Đơn vị:** Phần trăm (%).
  - **Tác dụng:** Tại độ sâu tìm kiếm >= 3, AI sẽ sắp xếp các nước đi và chỉ giữ lại một phần các nước đi tốt nhất, loại bỏ (cắt tỉa) các nước đi yếu hơn theo tỉ lệ này.
  - **Ví dụ:** Nếu Pruning Ratio là 50%, AI sẽ loại bỏ 50% số nước đi được đánh giá thấp nhất và chỉ tính toán 50% nước đi tốt nhất.
  - **Ảnh hưởng:** Tăng tỉ lệ này giúp AI tính toán nhanh hơn (đi sâu hơn) nhưng có rủi ro bỏ sót các nước đi chiến thuật bất ngờ (horizon effect).

## Cách chạy
1. Cài đặt Trunk: `cargo install trunk`
2. Chạy server: `trunk serve` hoăc `trunk serve --open` để mở trình duyệt ngay.
3. Mở `http://localhost:8080`
4. `trunk serve --release` để build release