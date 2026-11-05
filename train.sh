#!/usr/bin/env bash

# Script huấn luyện Reinforcement Learning (Self-Play + NNUE) cho GameCoTuong
# Sử dụng:
#   ./train.sh                  # Mặc định: 50 ván, 10 epochs
#   ./train.sh 100 20           # 100 ván, 20 epochs
#   ./train.sh --games 200 --epochs 30

set -e

# Giá trị mặc định
GAMES=50
EPOCHS=10
OUTPUT="cotuong_core/assets/default.nnue"

# Hỗ trợ truyền tham số vị trí hoặc cờ
if [[ "$1" =~ ^[0-9]+$ ]]; then
  GAMES="$1"
  if [[ "$2" =~ ^[0-9]+$ ]]; then
    EPOCHS="$2"
  fi
  EXTRA_ARGS=""
else
  EXTRA_ARGS="$@"
fi

echo "=========================================================="
echo " 🧠 Bắt đầu Huấn Luyện AI Cờ Tướng (NNUE Self-Play RL)     "
echo "=========================================================="
echo "🎯 Cấu hình:"
if [ -n "$EXTRA_ARGS" ]; then
  echo "  Tham số tùy chỉnh: $EXTRA_ARGS"
else
  echo "  Số ván Self-Play : $GAMES"
  echo "  Số Epochs        : $EPOCHS"
fi
echo "  File đích        : $OUTPUT"
echo ""

echo "▶ 1. Chạy quá trình Self-Play & Huấn luyện (cotuong_trainer)..."
if [ -n "$EXTRA_ARGS" ]; then
  cargo run --release -p cotuong_trainer -- --output "$OUTPUT" $EXTRA_ARGS
else
  cargo run --release -p cotuong_trainer -- --games "$GAMES" --epochs "$EPOCHS" --output "$OUTPUT"
fi
echo "✅ Huấn luyện và lượng tử hóa mô hình thành công!"
echo ""

echo "▶ 2. Kiểm thử mô hình NNUE trong cotuong_core..."
cargo test -p cotuong_core -- test_embedded_weights_validity test_incremental_accumulator_exactness
echo "✅ Kiểm thử weights hợp lệ!"
echo ""

echo "▶ 3. Biên dịch lại gói WebAssembly (WASM) với trọng số mới..."
npm run build:wasm
echo "✅ Cập nhật WASM hoàn tất!"
echo ""

echo "🎉 Quá trình huấn luyện và đóng gói hoàn thành thành công!"
echo "👉 Bạn có thể khởi động dev server (npm run dev) để trải nghiệm mô hình AI mới."

