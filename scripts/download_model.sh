#!/usr/bin/env bash
set -euo pipefail

MODEL_DIR="models/finbert"
REPO_ID="dark0ghost/ai-trader-bot-finbert"

echo "==> Downloading FinBERT ONNX model from HuggingFace Hub"
echo "    Repo: $REPO_ID"
echo "    Target: $MODEL_DIR"
echo ""

mkdir -p "$MODEL_DIR"

if ! command -v huggingface-cli &>/dev/null; then
    echo "[!] 'huggingface-cli' not found. Install with:"
    echo "    pip install huggingface_hub"
    exit 1
fi

huggingface-cli download "$REPO_ID" --local-dir "$MODEL_DIR" --local-dir-use-symlinks False

echo ""
echo "==> Done. Files in $MODEL_DIR:"
ls -lh "$MODEL_DIR"
