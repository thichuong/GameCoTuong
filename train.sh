#!/usr/bin/env bash

# ==============================================================================
#  🧠 Pipeline Tự Động Huấn Luyện AI Cờ Tướng (Xiangqi HalfKP NNUE)
# ==============================================================================
#  Tự động thực thi toàn bộ quy trình:
#    1. Huấn luyện Giám sát từ Kỳ Phổ Mẫu (Float 32-bit chính xác cao)
#    2. Chưng cất Tri thức cùng Sư phụ (Pikafish / Fairy-Stockfish nếu có)
#    3. Đấu tập Đối kháng (Sparring) để đánh giá năng lực thực chiến
#    4. LƯỢNG TỬ HÓA KẾT QUẢ CUỐI CÙNG sang int16/int8 (cotuong_core/assets/default.nnue)
#    5. Kiểm thử toàn diện & Biên dịch gói WebAssembly (cotuong_core.wasm)
# ==============================================================================

set -e

show_help() {
  echo "================================================================================"
  echo " 📖 HƯỚNG DẪN SỬ DỤNG SCRIPT HUẤN LUYỆN TOÀN BỘ (train.sh)"
  echo "================================================================================"
  echo "Cú pháp:"
  echo "  ./train.sh [TÙY_CHỌN]"
  echo ""
  echo "Mặc định (không tham số): Script sẽ tự động chạy toàn bộ các bước từ A đến Z."
  echo ""
  echo "Các tùy chọn chính:"
  echo "  --epochs <N>         : Số thế hệ huấn luyện mỗi giai đoạn (Mặc định: 5)"
  echo "  --dataset <FILE>     : Tệp kỳ phổ mẫu (Mặc định: cotuong_trainer/data/sample_master_games.txt)"
  echo "  --lr <RATE>          : Tốc độ học (Mặc định: 0.05)"
  echo "  --engine <PATH>      : Đường dẫn binary Engine sư phụ (Mặc định: tự tìm trong engines/)"
  echo "  --proto <ucci|uci>   : Giao thức engine ngoài (Mặc định: ucci)"
  echo "  --depth <N>          : Độ sâu phân tích của engine ngoài (Mặc định: 10)"
  echo "  --output <FILE>      : File NNUE lượng tử hóa đích (Mặc định: cotuong_core/assets/default.nnue)"
  echo "  --skip-distill       : Bỏ qua bước chưng cất tri thức với Engine sư phụ"
  echo "  --skip-sparring      : Bỏ qua bước đấu tập đối kháng"
  echo "  --skip-wasm          : Bỏ qua bước đóng gói WebAssembly"
  echo "  --scratch            : Huấn luyện mạng mới từ đầu (không nạp lại checkpoint cũ)"
  echo "================================================================================"
  exit 0
}

if [[ "$1" == "-h" || "$1" == "--help" ]]; then
  show_help
fi

# Thiết lập cấu hình mặc định
EPOCHS=5
DATASET="cotuong_trainer/data/sample_master_games.txt"
LR="0.05"
OUTPUT="cotuong_core/assets/default.nnue"
FLOAT_MODEL="checkpoints/model_latest.f32"
ENGINE_PATH=""
ENGINE_PROTO="ucci"
ENGINE_DEPTH=10
SKIP_DISTILL=0
SKIP_SPARRING=0
SKIP_WASM=0
EXTRA_FLAGS=()

# Đọc các đối số truyền vào
while [[ $# -gt 0 ]]; do
  case "$1" in
    --epochs)
      EPOCHS="$2"
      shift 2
      ;;
    --dataset)
      DATASET="$2"
      shift 2
      ;;
    --lr)
      LR="$2"
      shift 2
      ;;
    --output)
      OUTPUT="$2"
      shift 2
      ;;
    --engine)
      ENGINE_PATH="$2"
      shift 2
      ;;
    --proto)
      ENGINE_PROTO="$2"
      shift 2
      ;;
    --depth)
      ENGINE_DEPTH="$2"
      shift 2
      ;;
    --skip-distill)
      SKIP_DISTILL=1
      shift
      ;;
    --skip-sparring)
      SKIP_SPARRING=1
      shift
      ;;
    --skip-wasm)
      SKIP_WASM=1
      shift
      ;;
    --scratch)
      EXTRA_FLAGS+=("--scratch")
      shift
      ;;
    *)
      EXTRA_FLAGS+=("$1")
      shift
      ;;
  esac
done

mkdir -p checkpoints engines

echo "================================================================================"
echo " 🧠 Bắt đầu Tự Động Toàn Bộ Quy Trình Huấn Luyện AI Cờ Tướng (NNUE HalfKP)"
echo "================================================================================"
echo "⚙️  Cấu hình:"
echo "  • Số Epochs mỗi bước       : $EPOCHS"
echo "  • Tệp kỳ phổ mẫu           : $DATASET"
echo "  • Tốc độ học (LR)          : $LR"
echo "  • Tệp Float tạm thời       : $FLOAT_MODEL (Chưa lượng tử hóa)"
echo "  • Tệp Lượng tử hóa cuối    : $OUTPUT"
echo "================================================================================"
echo ""

# Tự động tìm engine ngoài nếu chưa chỉ định
if [ -z "$ENGINE_PATH" ]; then
  if [ -x "engines/pikafish/pikafish" ]; then
    ENGINE_PATH="engines/pikafish/pikafish"
    ENGINE_PROTO="ucci"
  elif [ -x "engines/fairy-stockfish/fairy-stockfish" ]; then
    ENGINE_PATH="engines/fairy-stockfish/fairy-stockfish"
    ENGINE_PROTO="uci"
  fi
fi

# ------------------------------------------------------------------------------
# BƯỚC 1: Huấn luyện có giám sát từ Kỳ Phổ Mẫu (Supervised Learning)
# ------------------------------------------------------------------------------
echo "▶ [1/5] Huấn luyện Giám sát từ Kỳ Phổ Mẫu (Lưu dạng Float f32 nguyên bản)..."
cargo run --release -p cotuong_trainer -- \
  --mode dataset \
  --dataset "$DATASET" \
  --epochs "$EPOCHS" \
  --lr "$LR" \
  --eval-games 10 \
  --checkpoints checkpoints \
  --output "$FLOAT_MODEL" \
  "${EXTRA_FLAGS[@]}"
echo "✅ Hoàn tất Giai đoạn 1 (Supervised Learning)!"
echo ""

# ------------------------------------------------------------------------------
# BƯỚC 2: Chưng cất Tri thức với Engine Sư Phụ (Nếu có engine)
# ------------------------------------------------------------------------------
if [ "$SKIP_DISTILL" -eq 0 ] && [ -n "$ENGINE_PATH" ] && [ -f "$ENGINE_PATH" ]; then
  echo "▶ [2/5] Chưng cất Tri thức cùng Engine Sư phụ: $ENGINE_PATH ($ENGINE_PROTO)..."
  cargo run --release -p cotuong_trainer -- \
    --mode teacher \
    --engine "$ENGINE_PATH" \
    --proto "$ENGINE_PROTO" \
    --depth "$ENGINE_DEPTH" \
    --dataset "$DATASET" \
    --epochs "$EPOCHS" \
    --lr "$LR" \
    --checkpoints checkpoints \
    --resume "$FLOAT_MODEL" \
    --output "$FLOAT_MODEL"
  echo "✅ Hoàn tất Giai đoạn 2 (Teacher Distillation)!"
else
  echo "ℹ️  [2/5] Bỏ qua bước chưng cất tri thức (Không tìm thấy engine ngoài hoặc đã gắn --skip-distill)."
fi
echo ""

# ------------------------------------------------------------------------------
# BƯỚC 3: Đấu tập Đối kháng (Sparring) để đánh giá năng lực thực tế
# ------------------------------------------------------------------------------
if [ "$SKIP_SPARRING" -eq 0 ] && [ -n "$ENGINE_PATH" ] && [ -f "$ENGINE_PATH" ]; then
  echo "▶ [3/5] Đấu tập Đối kháng (Sparring 6 ván) với Engine đối thủ..."
  cargo run --release -p cotuong_trainer -- \
    --mode sparring \
    --engine "$ENGINE_PATH" \
    --proto "$ENGINE_PROTO" \
    --games 6 \
    --depth 8 \
    --resume "$FLOAT_MODEL"
  echo "✅ Hoàn tất Giai đoạn 3 (Sparring Match)!"
else
  echo "ℹ️  [3/5] Bỏ qua bước đấu tập đối kháng (Không có engine hoặc đã gắn --skip-sparring)."
fi
echo ""

# ------------------------------------------------------------------------------
# BƯỚC 4: LƯỢNG TỬ HÓA KẾT QUẢ CUỐI CÙNG (Gọi hàm riêng --mode quantize)
# ------------------------------------------------------------------------------
echo "▶ [4/5] 🎯 LƯỢNG TỬ HÓA KẾT QUẢ CUỐI CÙNG: Float f32 -> Int16/Int8 NNUE..."
cargo run --release -p cotuong_trainer -- \
  --mode quantize \
  --input "$FLOAT_MODEL" \
  --output "$OUTPUT"
echo "✅ Đã lượng tử hóa và nén mô hình vào: $OUTPUT!"
echo ""

# ------------------------------------------------------------------------------
# BƯỚC 5: Kiểm thử Tính toàn vẹn & Đóng gói WebAssembly (WASM)
# ------------------------------------------------------------------------------
if [ "$SKIP_WASM" -eq 0 ]; then
  echo "▶ [5/5] Kiểm thử mô hình NNUE trong cotuong_core & Đóng gói WASM..."
  cargo test -p cotuong_core -- test_embedded_weights_validity test_accumulator_incremental_vs_refresh
  npm run build:wasm
  echo "✅ Đóng gói WebAssembly thành công!"
else
  echo "ℹ️  [5/5] Bỏ qua bước đóng gói WASM (--skip-wasm)."
fi
echo ""

echo "================================================================================"
echo "🎉 TOÀN BỘ PIPELINE HUẤN LUYỆN ĐÃ HOÀN TẤT THÀNH CÔNG RỰC RỠ!"
echo "👉 Bạn có thể khởi động dev server ('npm run dev') để trải nghiệm mô hình AI mới."
echo "================================================================================"
