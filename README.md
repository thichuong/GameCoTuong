# Cờ Tướng (Chinese Chess PWA)

Một ứng dụng Cờ Tướng chạy trên trình duyệt sử dụng Rust và WebAssembly (Leptos).

## Tính năng
- **Chế độ chơi:** Người vs Máy, Máy vs Máy (CvC), Người vs Người.
- **Cấu hình AI:** Tùy chỉnh tham số riêng biệt cho Đỏ và Đen.
- **Giao diện:** Đẹp mắt, hỗ trợ Dark Mode, Responsive (Mobile/Desktop).
- **PWA:** Hỗ trợ cài đặt và chạy offline.

## Kiến trúc dự án (Project Architecture)

Dự án được chia thành 2 phần chính trong một Cargo Workspace:

1.  **`cotuong_core`**: Thư viện chứa toàn bộ logic game, luật chơi, và engine AI. Thư viện này độc lập với giao diện và có thể được tái sử dụng cho các giao diện khác (CLI, GUI khác).
2.  **`CoTuong` (Root)**: Ứng dụng web sử dụng framework Leptos, đóng vai trò là giao diện người dùng (UI), kết nối với `cotuong_core`.

## Cấu Hình AI (Engine Parameters)

Bạn có thể tinh chỉnh sức mạnh và phong cách chơi của máy thông qua bảng cấu hình (Config Panel).

### 1. Giá trị quân cờ (Piece Values)
Điểm số cơ bản cho từng loại quân. AI sẽ ưu tiên bảo vệ quân có giá trị cao và đổi quân giá trị thấp lấy quân giá trị cao.
- **Tốt (Pawn):** Giá trị của quân Tốt (mặc định ~30).
- **Sĩ (Advisor):** Giá trị quân Sĩ (mặc định ~50).
- **Tượng (Elephant):** Giá trị quân Tượng (mặc định ~50).
- **Mã (Horse):** Giá trị quân Mã (mặc định ~100).
- **Pháo (Cannon):** Giá trị quân Pháo (mặc định ~110).
- **Xe (Rook):** Giá trị quân Xe (mặc định ~220).
- **Tướng (King):** Giá trị quân Tướng (rất lớn, mặc định ~2000).

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

- **Pruning Method (Phương pháp cắt tỉa):**
  - **Dynamic Limiting (Giới hạn động):** Giữ lại số lượng nước đi dựa trên công thức `8 + depth^2 * multiplier`.
    - **Multiplier (Hệ số nhân):** Điều chỉnh độ rộng của tìm kiếm (0.1 - 2.0). Giá trị càng cao càng giữ lại nhiều nước đi (an toàn hơn nhưng chậm hơn).
  - **Late Move Reductions (LMR):** Giảm độ sâu tìm kiếm cho các nước đi ở cuối danh sách.
  - **Both (Cả hai):** Kết hợp cả hai phương pháp để tối ưu tốc độ.

- **Depth Discount (Giảm điểm theo độ sâu):**
  - Tỉ lệ phần trăm điểm số được cộng thêm cho mỗi độ sâu (càng gần gốc càng được cộng nhiều).
  - Công thức: `score * (100 + discount * depth) / 100`.
  - Khuyến khích máy chọn các nước đi có lợi ngay lập tức (ở độ sâu lớn - gần gốc).

- **Mate Score (Điểm chiếu bí):**
  - Điểm thưởng cơ bản cho việc chiếu bí đối phương.
  - Giá trị càng cao, máy càng ưu tiên các biến dẫn đến chiếu bí nhanh nhất.

### 3. ProbCut (Cắt tỉa xác suất)
Các tham số cho kỹ thuật ProbCut (Probabilistic Cutpruning) giúp cắt tỉa mạnh mẽ các nhánh không hứa hẹn.
- **ProbCut Depth:** Độ sâu tối thiểu để áp dụng ProbCut.
- **ProbCut Margin:** Biên độ điểm số để quyết định cắt tỉa.
- **ProbCut Reduction:** Độ sâu giảm đi khi kiểm tra điều kiện cắt tỉa.

### 4. Hình phạt (Penalties)
- **Hanging Piece Penalty (Phạt quân treo):**
  - Điểm phạt khi một quân cờ bị tấn công mà không có quân bảo vệ (hoặc bị tấn công bởi quân giá trị thấp hơn).
  - Giúp máy cẩn thận hơn trong việc giữ quân.

### 5. Hệ thống (System)
- **TT Size (MB):** Kích thước bộ nhớ cho Bảng Băm (Transposition Table). Mặc định 256MB. Tăng lên giúp máy nhớ được nhiều thế cờ hơn.

## Cách chạy
1. Cài đặt Trunk: `cargo install trunk`
2. Chạy server: `trunk serve` hoăc `trunk serve --open` để mở trình duyệt ngay.
3. Mở `http://localhost:8080`
4. `trunk serve --release` để build release

## Phát triển (Development)

### Chạy Tests
Dự án bao gồm các unit tests cho logic game và engine. Để chạy tests:

```bash
cargo test --workspace
```

Hoặc chỉ chạy test cho phần core:

```bash
cargo test -p cotuong_core
```

### Cấu hình Engine qua JSON
Engine hỗ trợ tải cấu hình từ chuỗi JSON. Điều này hữu ích cho việc thử nghiệm các tham số khác nhau mà không cần biên dịch lại.

**Cấu trúc JSON mẫu:**

```json
{
  "val_pawn": 30,
  "val_advisor": 50,
  "val_elephant": 50,
  "val_horse": 100,
  "val_cannon": 110,
  "val_rook": 220,
  "val_king": 2000,
  "pst_pawn": [[1.0, ...]], 
  "score_hash_move": 200000,
  "score_capture_base": 200000,
  "score_killer_move": 120000,
  "score_history_max": 80000,
  "pruning_method": 0,
  "pruning_multiplier": 1.0
}
```

- Các giá trị `val_*` là điểm số quân cờ.
- `pst_*` là bảng điểm vị trí (Piece Square Tables), có thể là mảng 2 chiều `[[f32; 9]; 10]` để scale giá trị mặc định.
- `pruning_method`: 0 (Dynamic), 1 (LMR), 2 (Both).

### Benchmarks (Hiệu năng)

Để kiểm tra hiệu năng của Engine (NPS - Nodes Per Second), bạn có thể chạy lệnh sau:

```bash
cargo test --release -p cotuong_core -- engine::bench_test --nocapture
```

Lệnh này sẽ chạy các kịch bản test hiệu năng cho:
1.  **Khai cuộc (Opening):** Tìm kiếm ở độ sâu 5.
2.  **Tàn cuộc (Endgame):** Tìm kiếm ở độ sâu 7.

Kết quả sẽ hiển thị số nodes đã duyệt, thời gian thực thi và chỉ số NPS.