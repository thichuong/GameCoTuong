<p align="center">
  <img src="public/icon.png" alt="Cờ Tướng Logo" width="120"/>
</p>

<h1 align="center">🐉 Cờ Tướng (Chinese Chess PWA)</h1>

<p align="center">
  <strong>Ứng dụng Cờ Tướng chạy trên trình duyệt với AI mạnh mẽ, được xây dựng bằng Rust và WebAssembly</strong>
</p>

<p align="center">
  <a href="#tính-năng">Tính năng</a> •
  <a href="#cài-đặt">Cài đặt</a> •
  <a href="#kiến-trúc">Kiến trúc</a> •
  <a href="#cấu-hình-ai">Cấu hình AI</a> •
  <a href="#phát-triển">Phát triển</a>
</p>

---

## ✨ Tính năng

| Tính năng | Mô tả |
|-----------|-------|
| 🎮 **Đa chế độ chơi** | Người vs Máy, Máy vs Máy (CvC), Người vs Người (Offline/Online) |
| 🧠 **AI tùy biến** | Cấu hình riêng biệt cho quân Đỏ và quân Đen với hàng chục tham số |
| 🎨 **Giao diện hiện đại** | Dark Mode, Responsive (Mobile/Desktop) |
| 📱 **PWA** | Cài đặt như ứng dụng native, chạy offline |
| ⚡ **Hiệu suất cao** | Thuật toán Alpha-Beta với nhiều kỹ thuật tối ưu |
| 🔊 **Hiệu ứng âm thanh** | Di chuyển, Ăn quân, Chiếu tướng, Chiếu bí (Có âm thanh riêng biệt) |
| 💾 **Xuất dữ liệu** | Xuất biên bản ván đấu ra file CSV để phân tích |

---

## 🚀 Cài đặt

### Yêu cầu hệ thống

- **Rust** (stable, phiên bản 1.70+)
- **Trunk** - Build tool cho Rust WASM

### Hướng dẫn cài đặt

```bash
# 1. Cài đặt Trunk (nếu chưa có)
cargo install trunk

# 2. Clone repository
git clone https://github.com/username/GameCoTuong.git
cd GameCoTuong

# 3. Chạy development server
trunk serve --open

# 4. Mở trình duyệt tại http://localhost:8080
```

### Chế độ Production

```bash
# Build release với tối ưu hóa đầy đủ
trunk serve --release
```

### 🌐 Chạy Multiplayer (Local)

Để kiểm thử chế độ Online (2 người chơi trên 2 tab/máy):

   server lắng nghe trên port 3000:
   ```bash
   cargo run -p server
   ```

2. **Bước 2: Khởi động Client**
   client chạy trên port 8080:
   ```bash
   cd client && trunk serve
   # Mở 2 tab tại http://localhost:8080
   ```

3. **Bước 3: Bắt đầu game**
   - Chọn **"🌐 Chơi Online"** trong dropdown chế độ
   - Nhấn **"🎮 Tìm trận"** trên cả 2 tab
   - Server tự động ghép cặp và bắt đầu ván đấu

#### Tính năng Online Mode

| Tính năng | Mô tả |
|-----------|-------|
| 🔍 **Tìm trận** | Tự động ghép cặp 2 người chơi |
| ⏳ **Huỷ tìm** | Huỷ tìm trận khi đang chờ |
| 🔴⚫ **Lượt chơi** | Hiển thị rõ "Lượt của bạn" / "Đang chờ đối thủ" |
| 🏳️ **Đầu hàng** | Gửi thông báo đầu hàng, đối thủ thắng |
| 🏆 **Chiếu hết** | Server tự động phát hiện, thông báo kết quả |
| ⚠️ **Mất kết nối** | Thông báo khi đối thủ disconnect |
| 🎮 **Sẵn sàng** | Sau khi kết thúc, cả 2 nhấn "Sẵn sàng" để chơi tiếp |

---

## 🏗️ Kiến trúc

Dự án sử dụng **Cargo Workspace** với kiến trúc module hóa:

```
GameCoTuong/
├── cotuong_core/          # 📦 Core Library (Engine + Logic)
│   ├── src/
│   │   ├── engine/        # AI Engine (Alpha-Beta, Eval, TT)
│   │   │   ├── config.rs  # Cấu hình Engine
│   │   │   ├── search.rs  # Thuật toán tìm kiếm
│   │   │   ├── eval.rs    # Hàm đánh giá
│   │   │   ├── tt.rs      # Transposition Table
│   │   │   ├── zobrist.rs # Zobrist Hashing (nhận diện trạng thái bàn cờ)
│   │   │   └── move_list.rs # Quản lý danh sách nước đi tối ưu
│   │   └── logic/         # Luật chơi + Board
│   │       ├── board.rs   # Bàn cờ (Sử dụng BoardCoordinate an toàn)
│   │       ├── game.rs    # Game State
│   │       ├── rules.rs   # Luật di chuyển
│   │       ├── opening.rs # Khai cuộc (Opening Book)
│   │       └── lookup.rs  # Precomputed lookup tables
│   └── Cargo.toml
├── client/                # 🖥️ Web UI (Leptos Framework)
│   ├── src/
│   │   ├── app.rs         # Main Application
│   │   ├── components/    # UI Components
│   │   ├── network.rs     # WebSocket Client
│   │   └── main.rs        # Entry point
│   └── Cargo.toml
├── server/                # 🚀 WebSocket Server (Axum)
│   └── src/
│       ├── main.rs        # Server Entry point
│       ├── ws.rs          # WebSocket Handler
│       └── game_manager.rs # Game Logic & Matchmaking
├── shared/                # 🔗 Shared Types & Messages
│   └── src/
│       └── lib.rs         # Common Enums/Structs
└── Cargo.toml             # Workspace root
```

### Mô tả các module

| Module | Mô tả |
|--------|-------|
| `cotuong_core` | Thư viện độc lập chứa toàn bộ logic game và AI. Có thể tái sử dụng cho CLI, GUI khác. |
| `client` | Giao diện web sử dụng **Leptos** framework, biên dịch sang WebAssembly. |
| `server` | Backend server viết bằng **Axum**, xử lý WebSocket và ghép cặp người chơi. |
| `shared` | Thư viện dùng chung giữa client và server (định nghĩa các Message, GameState). |

---

## 🧠 Cấu hình AI (Engine Parameters)

Tinh chỉnh sức mạnh và phong cách chơi của máy thông qua **Config Panel** trong giao diện.

### 1. Tham số Tìm kiếm (Search Parameters)

| Tham số | Mô tả | Mặc định |
|---------|-------|----------|
| `score_hash_move` | Điểm thưởng cho nước đi từ Transposition Table | 200,000 |
| `score_capture_base` | Điểm thưởng cơ bản cho nước bắt quân (MVV-LVA) | 200,000 |
| `score_killer_move` | Điểm thưởng cho Killer Move (nước gây beta-cutoff) | 120,000 |
| `score_history_max` | Giới hạn điểm History Heuristic | 80,000 |
| `depth_discount` | % điểm cộng thêm mỗi độ sâu (ưu tiên lợi ích ngay) | 10 |
| `mate_score` | Điểm thưởng cho chiếu bí (càng cao càng ưu tiên) | 20,000 |

### 2. Phương pháp Cắt tỉa (Pruning)

| Tham số | Mô tả | Mặc định |
|---------|-------|----------|
| `pruning_method` | 0: Dynamic, 1: LMR, 2: Both | 1 (LMR) |
| `pruning_multiplier` | Hệ số nhân cho Dynamic Limiting (0.1 - 2.0) | 1.0 |

### 3. ProbCut (Cắt tỉa Xác suất)

| Tham số | Mô tả | Mặc định |
|---------|-------|----------|
| `probcut_depth` | Độ sâu tối thiểu để áp dụng ProbCut | 5 |
| `probcut_margin` | Biên độ điểm số để quyết định cắt tỉa | 200 |
| `probcut_reduction` | Độ sâu giảm khi kiểm tra điều kiện cắt tỉa | 4 |

### 4. Singular Extension

| Tham số | Mô tả | Mặc định |
|---------|-------|----------|
| `singular_extension_min_depth` | Độ sâu tối thiểu để áp dụng | 8 |
| `singular_extension_margin` | Biên độ xác định nước đi "singular" | 20 |

### 5. Hình phạt & Hệ thống

| Tham số | Mô tả | Mặc định |
|---------|-------|----------|
| `hanging_piece_penalty` | Phạt quân bị tấn công mà không được bảo vệ | 10 |
| `king_exposed_cannon_penalty` | Phạt tướng bị lộ mặt trước pháo (0 hoặc 1 quân chắn) | 20 |
| `tt_size_mb` | Kích thước Transposition Table (MB) | 256 |

---

## 🛠️ Phát triển (Development)

### Chạy Tests

```bash
# Chạy toàn bộ test trong workspace
cargo test --workspace

# Chạy test cho core library
cargo test -p cotuong_core

# Chạy test với output chi tiết
cargo test -p cotuong_core -- --nocapture

# Sử dụng script chạy toàn bộ
./test_all.sh
```

### Chạy Tests theo Module

```bash
# Test logic game (board, game state, rules)
cargo test -p cotuong_core logic::

# Test cụ thể cho board
cargo test -p cotuong_core logic::board::

# Test cụ thể cho game state
cargo test -p cotuong_core logic::game::

# Test engine (search, eval, config)
cargo test -p cotuong_core engine::

# Test config loading
cargo test -p cotuong_core engine::config::

# Test chiếu bí (checkmate)
cargo test -p cotuong_core engine::mate_test

# Test repetition (lặp nước đi)
cargo test -p cotuong_core logic::repetition_test

# Test phạt tướng lộ pháo
cargo test -p cotuong_core test_king_exposed_penalty

# Test bộ sinh nước đi (Move Generator)
cargo test -p cotuong_core logic::generator
```

### Chạy Test Cụ thể

```bash
# Chạy một test function cụ thể
cargo test -p cotuong_core test_load_config_default

# Chạy tests khớp pattern
cargo test -p cotuong_core -- "checkmate" --nocapture
```

### Benchmarks (Hiệu năng)

```bash
# Chạy benchmark để đo NPS (Nodes Per Second)
cargo test --release -p cotuong_core -- engine::bench_test --nocapture
```

Benchmark bao gồm:
- **Khai cuộc (Opening)**: Tìm kiếm ở độ sâu 5
- **Tàn cuộc (Endgame)**: Tìm kiếm ở độ sâu 7

Kết quả hiển thị: số nodes đã duyệt, thời gian thực thi, và chỉ số NPS.

---

## 📋 Cấu hình JSON

Engine hỗ trợ tải cấu hình từ file JSON. Sử dụng nút **Import/Export** trong giao diện.

### Cấu trúc JSON đầy đủ

```json
{
  "val_pawn": 1.0,
  "val_advisor": 1.0,
  "val_elephant": 1.0,
  "val_horse": 1.0,
  "val_cannon": 1.0,
  "val_rook": 1.0,
  "val_king": 1.0,

  "hanging_piece_penalty": 10,
  "king_exposed_cannon_penalty": 20,

  "score_hash_move": 1.0,
  "score_capture_base": 1.0,
  "score_killer_move": 1.0,
  "score_history_max": 1.0,
  "depth_discount": 10,
  "pruning_method": 1,
  "pruning_multiplier": 1.0,

  "probcut_depth": 5,
  "probcut_margin": 200,
  "probcut_reduction": 4,

  "singular_extension_min_depth": 8,
  "singular_extension_margin": 20,

  "mate_score": 20000,
  "tt_size_mb": 256
}
```

### Giải thích định dạng

> [!NOTE]
> - **Giá trị quân cờ** (`val_*`) và **điểm search** (`score_*`) là **hệ số scale** (float).
>   - `1.0` = giữ nguyên giá trị mặc định
>   - `1.5` = tăng 50%
>   - `0.5` = giảm 50%
> - Các tham số khác là **giá trị tuyệt đối**.

### Ví dụ: Tăng giá trị quân Xe

```json
{
  "val_rook": 1.5,
  "val_cannon": 0.8
}
```

Kết quả: Xe được đánh giá cao hơn 50%, Pháo thấp hơn 20%.

---

## 📦 Dependencies chính

| Package | Mô tả |
|---------|-------|
| [Leptos](https://leptos.dev/) | Reactive web framework cho Rust |
| [web-sys](https://rustwasm.github.io/wasm-bindgen/web-sys/index.html) | Bindings tới Web APIs |
| [serde](https://serde.rs/) | Serialization framework |
| [gloo-worker](https://docs.rs/gloo-worker) | Web Workers cho WASM |

---

## 📄 License

Dự án được phát hành dưới giấy phép **Open Font License**. Xem file [LICENSE](LICENSE) để biết thêm chi tiết.

---

<p align="center">
  Made with ❤️ and 🦀 Rust
</p>