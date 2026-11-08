<p align="center">
  <img src="public/icon.png" alt="Cờ Tướng Đại Sư Logo" width="110"/>
</p>

<h1 align="center">帥 Cờ Tướng Đại Sư (Xiangqi AI PWA)</h1>

<p align="center">
  <strong>Trò chơi Cờ Tướng (Xiangqi) truyền thống tích hợp AI HalfKP NNUE chuyên sâu chạy trên WebAssembly, xây dựng bằng Rust, React 18, TypeScript và Tailwind CSS v4</strong>
</p>

<p align="center">
  <a href="#-tính-năng-nổi-bật">Tính năng</a> •
  <a href="#-cài-đặt--khởi-chạy">Cài đặt</a> •
  <a href="#-kiến-trúc-hệ-thống">Kiến trúc</a> •
  <a href="#-tùy-chỉnh-tham-số-ai-engineconfig">Cấu hình AI</a> •
  <a href="#-huấn-luyện-mạng-nơ-ron-nnue-cotuong_trainer">Huấn luyện AI</a> •
  <a href="#-phát-triển--kiểm-thử">Phát triển</a>
</p>

---

## ✨ Tính năng Nổi bật

| Tính năng | Mô tả chi tiết |
|-----------|----------------|
| 🎮 **4 Chế độ chơi** | **Người đấu với Máy** (chọn cầm quân Đỏ đi tiên hoặc Đen đi hậu), **Máy đấu với Máy** (AI vs AI có nút Bắt đầu/Tạm dừng), **Hai người chơi** trên cùng thiết bị, và **Giải Cờ Thế (Puzzle Mode)** |
| 🧩 **Xếp & Giải Cờ Thế** | Tự do kéo-thả hoặc nhấp chọn quân từ khay vào bàn cờ, di chuyển quân trên bàn, click chuột phải để xóa quân, nạp **Thế cờ mẫu kinh điển**, kiểm tra hợp lệ thế cờ theo thời gian thực và Nhập/Xuất chuỗi **FEN** |
| 🧠 **AI Alpha-Beta + Dual-Head HalfKP NNUE v3** | Kết hợp tìm kiếm Negamax PVS, Iterative Deepening, Transposition Table, Null-Move Pruning, LMR, Sách khai cuộc cùng mạng nơ-ron **HalfKP NNUE v3** (`10,530 → 192×2 → 32 → 32 → 1` + 8-Bucket Phase PSQT) |
| 🎛️ **3 Biến thể Trọng số NNUE** | Chuyển đổi tức thì giữa **Float32 (`model_f32.bin` ~8.1MB)** chính xác tuyệt đối, **Int16 (`model_int16.nnue` ~4.0MB)** chuẩn SIMD cân bằng, và **Int8 (`model_int8.nnue` ~2.0MB)** siêu nén tiết kiệm RAM |
| 🎯 **5 Cấp độ khó** | Từ **Cấp 1 · Nhập môn (1s)** đến **Cấp 5 · Đại sư (20s)** điều chỉnh thời gian suy nghĩ tối đa mỗi nước của AI |
| ⚙️ **Tùy biến AI Đỏ & Đen độc lập** | Tinh chỉnh riêng hoặc đồng bộ cho từng bên (`tt_size_mb`, `mate_score`, `nnue_weight`, `model_type`) kèm 4 Preset định sẵn và Nhập/Xuất file `.json` |
| 🎨 **Bàn cờ Canvas 1:1 & Khay Quân Thích ứng** | Vẽ trên HTML5 Canvas chuẩn HiDPI/Retina, tự động giữ tỉ lệ vuông `1:1`, xoay chiều bàn cờ theo phe người chơi, hiển thị khay quân ăn được và ưu thế vật chất theo bố cục **Tự động / Hai bên / Trên dưới** |
| 📊 **HUD Tiến trình AI & Biên bản Ngăn kéo** | Lớp phủ HUD trực quan ngay trên bàn cờ khi AI đang tính nước; thanh biên bản gọn gàng phía dưới bàn cờ mở lên dạng ngăn kéo trượt (Slide-Up Drawer) |
| 🏋️ **Pipeline Huấn luyện NNUE (`cotuong_trainer`)** | Crate huấn luyện riêng bằng Rust hỗ trợ học giám sát từ kỳ phổ, chưng cất tri thức (Teacher Distillation) với Pikafish/Fairy-Stockfish (UCCI/UCI), đấu tập đối kháng và lượng tử hóa đa định dạng |
| 🔊 **Âm thanh, Xuất CSV & PWA** | Hiệu ứng âm thanh riêng cho đi quân, ăn quân, chiếu tướng, chiếu bí; xuất lịch sử ván đấu ra `.csv`; hỗ trợ cài đặt PWA và chơi offline |

---

## 🚀 Cài đặt & Khởi chạy

### Yêu cầu hệ thống

- **Node.js** (phiên bản 18+ hoặc 20+) & **npm**
- **Rust** (stable, phiên bản 1.75+) — *dùng khi biên dịch lại gói WASM từ `cotuong_core` hoặc chạy công cụ huấn luyện `cotuong_trainer`*
- **wasm-bindgen-cli** & target `wasm32-unknown-unknown` (khi cần build lại WASM từ Rust)

### 1. Chạy nhanh giao diện Web

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

### 3. Đóng gói & Triển khai Production

```bash
# Kiểm tra kiểu TypeScript và đóng gói ra thư mục dist/
npm run build

# Xem trước bản build production
npm run preview

# Triển khai lên Cloudflare Workers / Pages (qua Wrangler)
npm run deploy
```

---

## 🏗️ Kiến trúc Hệ thống

Dự án là một Rust Workspace gồm 2 crates (`cotuong_core` và `cotuong_trainer`) kết hợp cùng giao diện **React 18 + TypeScript + Tailwind CSS v4**:

```
GameCoTuong/
├── cotuong_core/                  # 🦀 Rust Core Library (Game Logic + AI Engine + NNUE + WASM)
│   ├── assets/                    # Trọng số mạng nơ-ron nhúng sẵn (default.nnue, model_f32.bin, model_int16.nnue, model_int8.nnue)
│   ├── src/
│   │   ├── lib.rs                 # Điểm xuất module: engine, logic, wasm
│   │   ├── wasm.rs                # Cầu nối #[wasm_bindgen]: WasmGameState & WasmEngine
│   │   ├── engine/                # Bộ tìm kiếm AI & Mạng lượng giá NNUE
│   │   │   ├── config.rs          # EngineConfig (mate_score, tt_size_mb, nnue_weight, model_type)
│   │   │   ├── search.rs          # AlphaBetaEngine (Negamax, PVS, Iterative Deepening, LMR, NMP)
│   │   │   ├── eval.rs            # SimpleEvaluator (điều phối trọng số NNUE & hệ số khuếch đại)
│   │   │   ├── nnue.rs            # Mạng Dual-Head HalfKP NNUE v3 (f32/int16/int8, Incremental Accumulator)
│   │   │   ├── movegen.rs         # EngineMoveGen (sinh nước đi kèm điểm Move Ordering)
│   │   │   ├── tt.rs              # Transposition Table (bảng băm thế cờ)
│   │   │   ├── zobrist.rs         # Zobrist Hashing 64-bit
│   │   │   └── move_list.rs       # Stack-allocated [Move; 128] (không cấp phát heap)
│   │   └── logic/                 # Luật Cờ Tướng & Biểu diễn Bàn cờ
│   │       ├── board.rs           # Board (Bitboard u128, cập nhật Zobrist hash & NNUE tăng dần)
│   │       ├── game.rs            # GameState (lượt đi, lịch sử, hoàn tác, phát hiện lặp nước 3 lần)
│   │       ├── generator.rs       # MoveGenerator (sinh nước đi hợp lệ)
│   │       ├── rules.rs           # Kiểm tra luật đi quân, tự chiếu, lộ mặt tướng
│   │       ├── lookup.rs          # AttackTables (bảng tra cứu tính sẵn với OnceLock)
│   │       ├── eval_constants.rs  # Hằng số điểm quân cờ & bảng vị trí (PST)
│   │       └── opening.rs         # Sách khai cuộc (Opening Book theo FEN)
│   └── tests/
│       └── mate_solving.rs        # Kiểm thử giải cờ thế / chiếu bí
├── cotuong_trainer/               # 🏋️ Rust NNUE Trainer CLI (Huấn luyện, Chưng cất & Lượng tử hóa)
│   ├── data/                      # Kho kỳ phổ mẫu (curated_master_games.txt, sample_master_games.txt)
│   └── src/
│       ├── main.rs                # FloatNnue, Backpropagation, Teacher Distillation, Sparring & Quantizer
│       ├── dataset.rs             # Trích xuất đặc trưng HalfKP & đọc dữ liệu FEN/PGN
│       └── protocol.rs            # Giao tiếp UCCI / UCI với Engine ngoài (Pikafish, Fairy-Stockfish)
├── src/                           # ⚛️ React 18 + TypeScript Frontend
│   ├── main.tsx                   # Entry point khởi tạo React
│   ├── App.tsx                    # Điều phối trạng thái ván cờ, 4 chế độ chơi, bố cục & Web Worker
│   ├── App.css                    # Tailwind CSS v4 & hiệu ứng chuyển động
│   ├── components/
│   │   ├── BoardCanvas.tsx        # Bàn cờ HTML5 Canvas 1:1, hỗ trợ kéo-thả xếp cờ thế & xoay góc nhìn
│   │   ├── CapturedPiecesTray.tsx # Khay quân đã ăn, điểm ưu thế & kho quân kéo-thả khi xếp cờ thế
│   │   ├── ControlsArea.tsx       # Thanh điều khiển chế độ chơi, công cụ cờ thế, chọn quân & độ khó
│   │   ├── PuzzleSetupArea.tsx    # Bảng công cụ xếp cờ thế, kiểm tra hợp lệ & nạp FEN/thế cờ mẫu
│   │   ├── ThinkingIndicator.tsx  # Lớp phủ HUD hiển thị tiến trình suy nghĩ của AI
│   │   ├── LogPanel.tsx           # Biên bản ván đấu thu gọn ở đáy & ngăn kéo trượt (Drawer)
│   │   └── ConfigModal.tsx        # Hộp thoại tùy chỉnh EngineConfig & chọn model f32/int16/int8
│   ├── workers/
│   │   └── ai.worker.ts           # Web Worker chạy tìm kiếm AI ở luồng nền
│   ├── types/
│   │   ├── game.ts                # Các interface TypeScript & DEFAULT_ENGINE_CONFIG
│   │   └── config.test.ts         # Kiểm thử đồng bộ cấu hình EngineConfig
│   ├── utils/
│   │   ├── capturedPieces.ts      # Tính toán quân bị ăn, ưu thế vật chất & kích thước bàn cờ tối ưu
│   │   ├── fen.ts                 # Chuyển đổi FEN, kiểm tra thế cờ hợp lệ & kho thế cờ kinh điển
│   │   ├── export.ts              # Xuất biên bản CSV & cấu hình JSON
│   │   └── sound.ts               # Quản lý âm thanh hiệu ứng
│   └── wasm/                      # Cầu nối WASM (cotuong_core.ts) & Polyfill Engine (enginePolyfill.ts)
├── public/                        # Tài nguyên tĩnh (manifest.json, sw.js, icon.png, sounds/)
├── train.sh                       # Script tự động hóa toàn bộ pipeline huấn luyện NNUE
├── train_ai.ipynb                 # Jupyter Notebook huấn luyện AI trên Google Colab
├── test_all.sh                    # Script chạy toàn bộ kiểm thử Rust & React build
└── wrangler.jsonc                 # Cấu hình triển khai Cloudflare Workers / Assets
```

> Xem tài liệu phân tích chi tiết các luồng dữ liệu, kiến trúc mạng NNUE và sơ đồ lớp tại [ARCHITECTURE.md](ARCHITECTURE.md).

---

## 🧠 Tùy chỉnh Tham số AI (`EngineConfig`)

Nhấn nút **"Cấu hình AI"** trên thanh công cụ để mở bảng tinh chỉnh chuyên sâu cho **AI Đỏ (Red)** và **AI Đen (Black)** (hỗ trợ bật/tắt **Đồng bộ 2 bên**). Các tham số khớp 100% với struct `EngineConfig` trong `cotuong_core`:

### 1. Các Tham số Cốt lõi

| Tham số | Kiểu dữ liệu | Mặc định | Phạm vi UI | Ý nghĩa |
|---------|-------------|----------|------------|---------|
| `tt_size_mb` | `usize` | `256` | `16 – 512 MB` | Dung lượng bảng băm Zobrist (Transposition Table) lưu trữ thế cờ đã duyệt và nhánh cắt tỉa |
| `mate_score` | `i32` | `300,000` | `50,000 – 500,000` | Điểm cơ sở khi phát hiện chiếu bí (`mate_score - ply`, càng sát nước thắng điểm càng cao) |
| `nnue_weight` | `i32` | `100` | `50% – 150%` | Hệ số khuếch đại độ nhạy điểm số của mạng nơ-ron HalfKP NNUE v3 theo công thức `(raw * nnue_weight) / 100` |
| `model_type` | `"f32" \| "int16" \| "int8"` | `"int16"` | 3 loại model | Chọn biến thể trọng số mạng nơ-ron (`f32` nguyên bản ~8.1MB, `int16` chuẩn SIMD ~4.0MB, `int8` siêu nén ~2.0MB) |

### 2. Các Preset Định sẵn

- **Tiêu Chuẩn (Mặc định)**: `tt_size_mb: 256`, `mate_score: 300000`, `nnue_weight: 100`, `model_type: "int16"`.
- **Tấn Công / Chuẩn Xác Cao**: `tt_size_mb: 256`, `mate_score: 400000`, `nnue_weight: 120`, `model_type: "f32"`.
- **Tiết Kiệm RAM**: `tt_size_mb: 64`, `mate_score: 200000`, `nnue_weight: 90`, `model_type: "int8"`.
- **Sát Cục / Giải Cờ Thế**: `tt_size_mb: 512`, `mate_score: 500000`, `nnue_weight: 110`, `model_type: "int16"`.

### 3. Định dạng JSON Nhập / Xuất

```json
{
  "mate_score": 300000,
  "tt_size_mb": 256,
  "nnue_weight": 100,
  "model_type": "int16"
}
```

---

## 🏋️ Huấn luyện Mạng Nơ-ron NNUE (`cotuong_trainer`)

Dự án tích hợp sẵn pipeline huấn luyện mạng **Dual-Head HalfKP NNUE v3** bằng Rust (`cotuong_trainer`) và script tự động `train.sh`:

```bash
# Chạy toàn bộ quy trình huấn luyện tự động (Kỳ phổ mẫu -> Chưng cất -> Đấu tập -> Lượng tử hóa -> Build WASM)
npm run train

# Hoặc tùy chỉnh tham số trực tiếp qua train.sh
./train.sh --epochs 10 --lr 0.05 --dataset cotuong_trainer/data/curated_master_games.txt
```

Ngoài ra, bạn có thể mở file [`train_ai.ipynb`](train_ai.ipynb) trên **Google Colab** để huấn luyện với GPU/CPU nhiều nhân và tải về các file trọng số `default.nnue`, `model_int16.nnue`, `model_int8.nnue`, `model_f32.bin`.

---

## 🛠️ Phát triển & Kiểm thử

### Chạy toàn bộ quy trình kiểm tra tự động

Script `test_all.sh` kiểm tra định dạng code Rust, chạy Clippy nghiêm ngặt cho cả native lẫn `wasm32`, chạy toàn bộ unit tests và kiểm tra quá trình build React/TypeScript:

```bash
./test_all.sh
```

### Các lệnh kiểm thử Rust (`cotuong_core` & `cotuong_trainer`)

```bash
# Chạy tất cả unit tests và bài test giải cờ thế trong toàn workspace
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
# Chạy unit tests TypeScript (FEN, Captured Pieces, EngineConfig, Polyfill Engine)
npm test

# Kiểm tra kiểu tĩnh TypeScript
npm run lint

# Build bản phân phối Vite
npm run build
```

---

## 📦 Công nghệ Sử dụng

| Công nghệ | Vai trò |
|-----------|---------|
| **Rust (Edition 2021)** | Ngôn ngữ xây dựng Core Engine (`cotuong_core`), mạng HalfKP NNUE v3, Bitboard `u128` và công cụ huấn luyện (`cotuong_trainer`) |
| **wasm-bindgen & serde-wasm-bindgen** | Cầu nối giao tiếp tốc độ cao giữa Rust/WASM và TypeScript |
| **React 18 & TypeScript 5** | Xây dựng giao diện người dùng, quản lý trạng thái, chế độ Cờ Thế và tương tác kéo-thả |
| **Tailwind CSS v4 & Lucide Icons** | Thiết kế giao diện sơn mài cổ điển và bố cục thích ứng đa thiết bị |
| **Vite 5 & Wrangler** | Công cụ đóng gói phát triển tốc độ cao và triển khai Cloudflare Workers |

---

## 📄 Giấy phép (License)

Dự án được phát hành theo giấy phép trong file [LICENSE](LICENSE).