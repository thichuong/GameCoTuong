#!/usr/bin/env bash

# ==============================================================================
#  🧠 Script Huấn Luyện AI Cờ Tướng (Kỳ Phổ Mẫu & Tích Hợp Engine Ngoài)
# ==============================================================================
#  Kiến trúc mạng : HalfKP(10.530 -> 192) x 2 -> 32 -> 32 -> 1
#  Các chế độ:
#    1. dataset  : Huấn luyện có giám sát (SL) từ kỳ phổ / ván cờ mẫu danh thủ (Mặc định)
#    2. teacher  : Gán nhãn đánh giá sâu từ Engine sư phụ (Pikafish / Fairy-Stockfish)
#    3. sparring : Đấu tập đối kháng trực tiếp với Engine ngoài
#    4. selfplay : Tự đấu tăng cường (RL Self-Play) theo giải thuật nội bộ
# ==============================================================================

set -e

show_help() {
  echo "================================================================================"
  echo " 📖 HƯỚNG DẪN SỬ DỤNG SCRIPT HUẤN LUYỆN (train.sh)"
  echo "================================================================================"
  echo "Cú pháp:"
  echo "  ./train.sh [CHẾ_ĐỘ] [TÙY_CHỌN]"
  echo ""
  echo "Các chế độ chính (--mode):"
  echo "  • dataset (Mặc định) : Huấn luyện từ kỳ phổ mẫu / thế cờ danh thủ"
  echo "      ./train.sh --mode dataset --epochs 5 --dataset cotuong_trainer/data/sample_master_games.txt"
  echo ""
  echo "  • teacher           : Dùng Engine ngoài (Pikafish/Fairy-Stockfish) làm sư phụ gán nhãn"
  echo "      ./train.sh --mode teacher --engine /path/to/pikafish --depth 12"
  echo ""
  echo "  • sparring          : Cho Bot đấu tập trực tiếp với Engine ngoài"
  echo "      ./train.sh --mode sparring --engine /path/to/pikafish --games 20"
  echo ""
  echo "  • selfplay          : Tự đấu sinh ván cờ bằng giải thuật nội bộ"
  echo "      ./train.sh --mode selfplay --games 30 --epochs 5"
  echo ""
  echo "Các cờ tùy chọn:"
  echo "  --dataset <FILE>    : Đường dẫn file kỳ phổ mẫu (Mặc định: sample_master_games.txt)"
  echo "  --engine <PATH>     : Đường dẫn file binary Engine cờ tướng ngoài"
  echo "  --proto <ucci|uci>  : Giao thức engine ngoài (Mặc định: ucci)"
  echo "  --depth <N>         : Độ sâu phân tích của engine ngoài (Mặc định: 10)"
  echo "  --epochs <M>        : Số thế hệ huấn luyện (Mặc định: 5)"
  echo "  --lr <RATE>         : Tốc độ học (Learning rate, mặc định: 0.01)"
  echo "  --output <FILE>     : File nhị phân lưu trọng số (Mặc định: cotuong_core/assets/default.nnue)"
  echo "================================================================================"
  exit 0
}

if [[ "$1" == "-h" || "$1" == "--help" ]]; then
  show_help
fi

# Giá trị mặc định
OUTPUT="cotuong_core/assets/default.nnue"
mkdir -p checkpoints engines

echo "================================================================================"
echo " 🧠 Bắt đầu Huấn Luyện AI Cờ Tướng (NNUE HalfKP Master Dataset & External Engine)"
echo "================================================================================"
echo "🎯 Cấu hình:"
echo "  • Kiến trúc mạng           : HalfKP(10.530 -> 192) x 2 -> 32 -> 32 -> 1"
echo "  • File trọng số đích       : $OUTPUT"
echo "  • Tham số thực thi         : ${*:-'(Mặc định: --mode dataset --epochs 5)'}"
echo "================================================================================"
echo ""

echo "▶ 1. Chạy quá trình huấn luyện AI (cotuong_trainer)..."
if [ $# -eq 0 ]; then
  cargo run --release -p cotuong_trainer -- --mode dataset --epochs 5 --output "$OUTPUT"
else
  cargo run --release -p cotuong_trainer -- --output "$OUTPUT" "$@"
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
