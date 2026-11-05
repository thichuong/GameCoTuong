#!/usr/bin/env bash

# ==============================================================================
#  🧠 Script Huấn Luyện Reinforcement Learning (Self-Play + NNUE HalfKP)
# ==============================================================================
#  Kiến trúc mạng : HalfKP(10.530 -> 192) x 2 -> 32 -> 32 -> 1
#  Cơ chế         : Self-Play sinh thế cờ -> SGD Replay Buffer -> Gating Tournament
# ==============================================================================

set -e

# Hiển thị hướng dẫn sử dụng chi tiết
show_help() {
  echo "================================================================================"
  echo " 📖 HƯỚNG DẪN SỬ DỤNG SCRIPT HUẤN LUYỆN (train.sh)"
  echo "================================================================================"
  echo "Cú pháp:"
  echo "  ./train.sh [SỐ_VÁN_SELFPLAY] [SỐ_EPOCHS] [SỐ_VÁN_ĐẤU_ĐÁNH_GIÁ]"
  echo "  hoặc dùng cờ lệnh trực tiếp:"
  echo "  ./train.sh --games <N> --epochs <M> --eval-games <K> [--output <FILE>]"
  echo ""
  echo "Các tham số chính:"
  echo "  1. SỐ_VÁN_SELFPLAY     : Số ván tự đấu mỗi Epoch để sinh mẫu cờ (Mặc định: 30)"
  echo "  2. SỐ_EPOCHS           : Số thế hệ / vòng lặp huấn luyện (Mặc định: 5)"
  echo "  3. SỐ_VÁN_ĐẤU_ĐÁNH_GIÁ : Số ván Candidate đấu với Champion (Mặc định: 10, tráo màu 50/50)"
  echo "                           (Nếu Candidate thắng >= 50-52%, trọng số mới sẽ được chấp nhận)"
  echo "  --output <ĐƯỜNG_DẪN>   : File nhị phân lưu weights (Mặc định: cotuong_core/assets/default.nnue)"
  echo ""
  echo "Các chế độ chạy khuyến nghị:"
  echo "  • Chạy kiểm tra nhanh (Test/Debug) :"
  echo "      ./train.sh 4 2 2"
  echo "  • Huấn luyện tiêu chuẩn (Khuyến nghị hàng ngày):"
  echo "      ./train.sh 30 5 10"
  echo "  • Huấn luyện chuyên sâu (Chạy qua đêm / nhiều tài nguyên):"
  echo "      ./train.sh 100 20 20"
  echo ""
  echo "Quy trình tự động sau khi train:"
  echo "  1. Tự động lượng tử hóa (Quantization) và lưu trọng số."
  echo "  2. Chạy unit tests kiểm tra tính hợp lệ và độ nhất quán của Accumulator."
  echo "  3. Tự động biên dịch lại gói WASM (cotuong_core.wasm) cho giao diện web."
  echo "================================================================================"
  exit 0
}

# Kiểm tra cờ yêu cầu trợ giúp
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
  show_help
fi

# Giá trị mặc định
GAMES=30
EPOCHS=5
EVAL_GAMES=10
OUTPUT="cotuong_core/assets/default.nnue"

# Hỗ trợ truyền tham số vị trí hoặc cờ
if [[ "$1" =~ ^[0-9]+$ ]]; then
  GAMES="$1"
  if [[ "$2" =~ ^[0-9]+$ ]]; then
    EPOCHS="$2"
  fi
  if [[ "$3" =~ ^[0-9]+$ ]]; then
    EVAL_GAMES="$3"
  fi
  EXTRA_ARGS=""
else
  EXTRA_ARGS="$@"
fi

echo "================================================================================"
echo " 🧠 Bắt đầu Huấn Luyện AI Cờ Tướng (NNUE HalfKP Self-Play RL)                   "
echo "================================================================================"
echo "🎯 Cấu hình:"
echo "  • Kiến trúc mạng           : HalfKP(10.530 -> 192) x 2 -> 32 -> 32 -> 1"
if [ -n "$EXTRA_ARGS" ]; then
  echo "  • Tham số tùy chỉnh        : $EXTRA_ARGS"
else
  echo "  • Số ván Self-Play/Epoch   : $GAMES"
  echo "  • Số Epochs (Thế hệ)       : $EPOCHS"
  echo "  • Số ván Đấu Đánh Giá      : $EVAL_GAMES (Gating Candidate vs Champion)"
fi
echo "  • File trọng số đích       : $OUTPUT"
echo "================================================================================"
echo ""

echo "▶ 1. Chạy quá trình Self-Play, Huấn luyện & Giải đấu Đánh giá (cotuong_trainer)..."
if [ -n "$EXTRA_ARGS" ]; then
  cargo run --release -p cotuong_trainer -- --output "$OUTPUT" $EXTRA_ARGS
else
  cargo run --release -p cotuong_trainer -- --games "$GAMES" --epochs "$EPOCHS" --eval-games "$EVAL_GAMES" --output "$OUTPUT"
fi
echo "✅ Huấn luyện và lượng tử hóa mô hình thành công!"
echo ""

echo "▶ 2. Kiểm thử mô hình NNUE trong cotuong_core..."
cargo test -p cotuong_core -- test_embedded_weights_validity test_accumulator_incremental_vs_refresh
echo "✅ Kiểm thử weights và accumulator hợp lệ!"
echo ""

echo "▶ 3. Biên dịch lại gói WebAssembly (WASM) với trọng số mới..."
npm run build:wasm
echo "✅ Cập nhật WASM hoàn tất!"
echo ""

echo "🎉 Quá trình huấn luyện và đóng gói hoàn thành thành công!"
echo "👉 Bạn có thể khởi động dev server (npm run dev) để trải nghiệm mô hình AI mới."

