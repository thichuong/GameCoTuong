<p align="center">
  <img src="public/icon.png" alt="Cờ Tướng Đại Sư Logo" width="110"/>
</p>

<h1 align="center">帥 Cờ Tướng Đại Sư (Xiangqi AI PWA)</h1>

<p align="center">
  <strong>Trò chơi Cờ Tướng (Xiangqi) truyền thống tích hợp AI thông minh chạy trên WebAssembly, xây dựng bằng Rust, React 18, TypeScript và Tailwind CSS</strong>
</p>

<p align="center">
  <a href="#-tính-năng-nổi-bật">Tính năng</a> •
  <a href="#-cài-đặt--khởi-chạy">Cài đặt</a> •
  <a href="#-kiến-trúc-hệ-thống">Kiến trúc</a> •
  <a href="#-tùy-chỉnh-tham-số-ai">Cấu hình AI</a> •
  <a href="#-phát-triển--kiểm-thử">Phát triển</a>
</p>

---

## ✨ Tính năng Nổi bật

| Tính năng | Mô tả chi tiết |
|-----------|----------------|
| 🎮 **3 Chế độ chơi** | **Người đấu với Máy** (chọn cầm quân Đỏ đi tiên hoặc Đen đi hậu), **Máy đấu với Máy** (AI vs AI có nút Bắt đầu/Tạm dừng), và **Hai người chơi** trên cùng thiết bị |
| 🧠 **AI Alpha-Beta Chuyên sâu** | Tích hợp Iterative Deepening, Principal Variation Search (PVS), Transposition Table, Null-Move Pruning, ProbCut, Singular Extension, LMR và Sách khai cuộc (Opening Book) |
| 🎯 **5 Cấp độ khó** | Từ **Mức 1 · Nhập môn (1s)** đến **Mức 5 · Đại sư (20s)** điều chỉnh thời gian suy nghĩ tối đa mỗi nước của AI |
| ⚙️ **Tùy biến AI Đỏ & Đen độc lập** | Tinh chỉnh riêng biệt giá trị 7 loại quân cờ, trọng số sắp xếp nước đi và phương pháp cắt tỉa cho từng bên (rất hữu ích khi cho 2 cấu hình AI tự đấu với nhau) |
| 🎨 **Bàn cờ Canvas 1:1 Sắc nét** | Vẽ trên HTML5 Canvas chuẩn HiDPI/Retina, tự động giữ tỉ lệ vuông `1:1` theo kích thước màn hình và tự động xoay chiều bàn cờ theo phe người chơi |
| 📊 **HUD Tiến trình AI Trực quan** | Hiển thị lớp phủ (Overlay HUD) ngay trên bàn cờ khi AI đang tính nước kèm đồng hồ thời gian thực, cấp độ và thống kê độ sâu / số nút duyệt (`nodes`) |
| 📜 **Biên bản Ngăn kéo Thu gọn** | Thanh biên bản gọn gàng phía dưới bàn cờ (*ẩn xuống, khi nào cần thì mở lên xem*), ghi lại chi tiết tọa độ, quân ăn được và thông số suy nghĩ của AI |
| 💾 **Xuất CSV & Nhập/Xuất JSON** | Xuất toàn bộ lịch sử ván đấu ra file `.csv` để phân tích; lưu và nạp cấu hình tham số AI qua file `.json` |
| 🔊 **Âm thanh & Hỗ trợ PWA** | Hiệu ứng âm thanh riêng biệt cho di chuyển, ăn quân, chiếu tướng, chiếu bí; hỗ trợ cài đặt như ứng dụng độc lập (PWA) và chơi offline |

---

## 🚀 Cài đặt & Khởi chạy

### Yêu cầu hệ thống

- **Node.js** (phiên bản 18+ hoặc 20+) & **npm**
- **Rust** (stable, phiên bản 1.75+) — *chỉ bắt buộc nếu bạn muốn sửa code Rust và biên dịch lại gói WASM; kho mã nguồn đã kèm sẵn bản build WASM trong `src/wasm/`*
- **wasm-bindgen-cli** & target `wasm32-unknown-unknown` (khi cần build lại WASM từ Rust)

### 1. Chạy nhanh giao diện Web (Sử dụng gói WASM có sẵn)

```bash
# Cài đặt dependencies
npm install

# Khởi chạy máy chủ phát triển (mặc định tại http://localhost:3000)
npm run dev
```

### 2. Biên dịch lại Rust Core sang WebAssembly (Tùy chọn)

Nếu bạn chỉnh sửa mã nguồn trong thư mục `cotuong_core/`, hãy chạy lệnh sau để cập nhật lại module WASM trong `src/wasm/`:

```bash
# Thêm target wasm32 (nếu chưa có)
rustup target add wasm32-unknown-unknown

# Biên dịch cotuong_core ở chế độ release và tạo bindings cho trình duyệt
npm run build:wasm
```

### 3. Đóng gói bản Production

```bash
# Kiểm tra kiểu TypeScript và đóng gói ra thư mục dist/
npm run build

# Xem trước bản build production
npm run preview
```

---

## 🏗️ Kiến trúc Hệ thống

Dự án kết hợp sức mạnh tính toán tốc độ cao của **Rust (`cotuong_core`)** được biên dịch sang **WebAssembly** cùng giao diện hiện đại viết bằng **React 18 + TypeScript + Tailwind CSS v4**:

```
GameCoTuong/
├── cotuong_core/                  # 🦀 Rust Core Library (Game Logic + AI Engine + WASM)
│   ├── src/
│   │   ├── lib.rs                 # Điểm xuất module: engine, logic, wasm
│   │   ├── wasm.rs                # Cầu nối #[wasm_bindgen]: WasmGameState & WasmEngine
│   │   ├── engine/                # Bộ tìm kiếm AI
│   │   │   ├── config.rs          # EngineConfig (tham số lượng giá & cắt tỉa)
│   │   │   ├── search.rs          # AlphaBetaEngine (Negamax, PVS, Iterative Deepening)
│   │   │   ├── eval.rs            # SimpleEvaluator (Material, PST, Mobility, King Safety)
│   │   │   ├── movegen.rs         # EngineMoveGen (sinh nước đi kèm điểm Move Ordering)
│   │   │   ├── tt.rs              # Transposition Table (bảng băm thế cờ)
│   │   │   ├── zobrist.rs         # Zobrist Hashing 64-bit
│   │   │   └── move_list.rs       # Stack-allocated [Move; 128] (không cấp phát heap)
│   │   └── logic/                 # Luật Cờ Tướng & Biểu diễn Bàn cờ
│   │       ├── board.rs           # Board (Bitboard u128, cập nhật hash & điểm tăng dần)
│   │       ├── game.rs            # GameState (lượt đi, lịch sử, hoàn tác, phát hiện lặp nước)
│   │       ├── generator.rs       # MoveGenerator (sinh nước đi hợp lệ)
│   │       ├── rules.rs           # Kiểm tra luật đi quân, tự chiếu, lộ mặt tướng
│   │       ├── lookup.rs          # AttackTables (bảng tra cứu tính sẵn với OnceLock)
│   │       ├── eval_constants.rs  # Hằng số điểm quân cờ & bảng vị trí (PST)
│   │       └── opening.rs         # Sách khai cuộc (Opening Book theo FEN)
│   └── tests/
│       └── mate_solving.rs        # Kiểm thử giải cờ thế / chiếu bí
├── src/                           # ⚛️ React 18 + TypeScript Frontend
│   ├── main.tsx                   # Entry point khởi tạo React
│   ├── App.tsx                    # Điều phối trạng thái ván cờ, chế độ chơi & Web Worker
│   ├── App.css                    # Tailwind CSS v4 & hiệu ứng chuyển động
│   ├── components/
│   │   ├── BoardCanvas.tsx        # Bàn cờ HTML5 Canvas 1:1 tự động co giãn & xoay góc nhìn
│   │   ├── ControlsArea.tsx       # Thanh điều khiển chế độ, chọn quân, độ khó & tác vụ
│   │   ├── ThinkingIndicator.tsx  # Lớp phủ HUD hiển thị tiến trình suy nghĩ của AI
│   │   ├── LogPanel.tsx           # Biên bản ván đấu thu gọn ở đáy & ngăn kéo trượt (Drawer)
│   │   └── ConfigModal.tsx        # Hộp thoại tùy chỉnh tham số AI cho Đỏ và Đen
│   ├── workers/
│   │   └── ai.worker.ts           # Web Worker chạy tìm kiếm AI ở luồng nền
│   ├── types/
│   │   └── game.ts                # Các interface TypeScript & cấu hình AI mặc định
│   ├── utils/
│   │   ├── export.ts              # Xuất biên bản CSV & cấu hình JSON
│   │   └── sound.ts               # Quản lý âm thanh hiệu ứng
│   └── wasm/                      # File kết nối JS/TS và binary .wasm sinh bởi wasm-bindgen
├── public/                        # Tài nguyên tĩnh (manifest.json, sw.js, icon.png, sounds/)
├── Cargo.toml                     # Cấu hình Rust Workspace & bộ quy tắc Clippy nghiêm ngặt
├── package.json                   # Cấu hình scripts & thư viện Frontend
└── test_all.sh                    # Script chạy toàn bộ kiểm thử Rust & React build
```

> Xem tài liệu phân tích chi tiết các luồng dữ liệu và sơ đồ lớp tại [ARCHITECTURE.md](ARCHITECTURE.md).

---

## 🧠 Tùy chỉnh Tham số AI (Engine Configuration)

Nhấn nút **"Cấu hình AI"** trên thanh công cụ để mở bảng tinh chỉnh chuyên sâu riêng biệt cho **AI Đỏ (Red)** và **AI Đen (Black)**. Bạn có thể cho hai máy có phong cách khác nhau (ví dụ: một bên thiên về tấn công ăn quân, một bên ưu tiên giữ Xe/Pháo) tự thi đấu với nhau ở chế độ **Máy đấu với Máy**.

### 1. Giá trị Quân cờ Cơ bản (`val_*`)

| Tham số | Quân cờ | Giá trị mặc định UI | Phạm vi điều chỉnh |
|---------|---------|---------------------|--------------------|
| `val_pawn` | Tốt / Binh (`兵` / `卒`) | `30` | `0 – 200` |
| `val_advisor` | Sĩ (`仕` / `士`) | `120` | `0 – 400` |
| `val_elephant` | Tượng (`相` / `象`) | `120` | `0 – 400` |
| `val_horse` | Mã (`傌` / `馬`) | `300` | `0 – 800` |
| `val_cannon` | Pháo (`炮` / `砲`) | `300` | `0 – 900` |
| `val_rook` | Xe (`俥` / `車`) | `600` | `0 – 1,800` |
| `val_king` | Tướng (`帥` / `將`) | `10,000` | `5,000 – 20,000` |

### 2. Trọng số Sắp xếp Nước đi & Cắt tỉa

| Tham số | Ý nghĩa | Mặc định UI |
|---------|---------|-------------|
| `score_hash_move` | Điểm ưu tiên cho nước đi tốt nhất từ Transposition Table | `3,000,000` |
| `score_capture_base` | Điểm thưởng nền cho các nước ăn quân (kết hợp MVV-LVA) | `1,000,000` |
| `score_killer_move` | Điểm ưu tiên cho Killer Move (nước yên lặng gây cắt tỉa $\beta$) | `900,000` |
| `score_history_max` | Giới hạn điểm tối đa của bảng History Heuristic | `800,000` |
| `pruning_method` | `0`: Dynamic Limiting, `1`: Late Move Reductions (LMR), `2`: Kết hợp cả hai | `0` |
| `pruning_multiplier` | Hệ số nhân điều chỉnh mức độ cắt tỉa (`0.1` – `2.0`) | `1.0` |
| `mate_score` | Điểm thưởng cho thế chiếu bí (`10,000` – `50,000`) | `30,000` |

### 3. Định dạng JSON Nhập / Xuất

Bạn có thể xuất cấu hình hiện tại ra file `.json` hoặc tải lên một file cấu hình tùy chỉnh:

```json
{
  "val_pawn": 30,
  "val_advisor": 120,
  "val_elephant": 120,
  "val_horse": 300,
  "val_cannon": 300,
  "val_rook": 600,
  "val_king": 10000,
  "score_hash_move": 3000000,
  "score_capture_base": 1000000,
  "score_killer_move": 900000,
  "score_history_max": 800000,
  "pruning_method": 1,
  "pruning_multiplier": 1.0,
  "mate_score": 30000
}
```

---

## 🛠️ Phát triển & Kiểm thử

### Chạy toàn bộ quy trình kiểm tra tự động

Script `test_all.sh` kiểm tra định dạng code Rust, chạy Clippy nghiêm ngặt cho cả native lẫn `wasm32`, chạy toàn bộ unit tests và kiểm tra quá trình build React/TypeScript:

```bash
./test_all.sh
```

### Các lệnh kiểm thử Rust (`cotuong_core`)

```bash
# Chạy tất cả unit tests và bài test giải cờ thế
cargo test --workspace

# Chạy kiểm tra theo từng phân hệ
cargo test -p cotuong_core logic::
cargo test -p cotuong_core engine::
cargo test -p cotuong_core --test mate_solving

# Chạy benchmark đo tốc độ duyệt nút (NPS - Nodes Per Second) ở chế độ Release
cargo test --release -p cotuong_core -- engine::bench_test --nocapture

# Kiểm tra định dạng và Clippy lints
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown --all-targets -- -D warnings
```

### Các lệnh kiểm tra Frontend (`src/`)

```bash
# Kiểm tra kiểu tĩnh TypeScript
npm run lint

# Build thử bản phân phối Vite
npm run build
```

---

## 📦 Công nghệ Sử dụng

| Công nghệ | Vai trò |
|-----------|---------|
| **Rust (Edition 2021)** | Ngôn ngữ xây dựng Core Engine, biểu diễn Bitboard `u128` và luật Cờ Tướng |
| **wasm-bindgen & serde-wasm-bindgen** | Cầu nối giao tiếp tốc độ cao giữa Rust/WASM và TypeScript |
| **React 18 & TypeScript 5** | Xây dựng giao diện người dùng, quản lý trạng thái và tương tác |
| **Tailwind CSS v4 & Lucide Icons** | Thiết kế giao diện cổ điển, hiện đại và đáp ứng mọi kích thước màn hình |
| **Vite 5** | Công cụ đóng gói và máy chủ phát triển tốc độ cao |

---

## 📄 Giấy phép (License)

Dự án được phát hành theo giấy phép trong file [LICENSE](LICENSE).