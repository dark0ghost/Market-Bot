#!/bin/bash
set -euo pipefail

# Market Bot — Full Training Pipeline
# Usage: HF_TOKEN=hf_xxx ./training/pipeline.sh [--days 30]

DAYS=7
DOCKER_IMAGE="${DOCKER_IMAGE:-finbert-sft:latest}"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --days) DAYS="$2"; shift 2 ;;
        *) shift ;;
    esac
done

echo "========================================================"
echo "  Market Bot — Sberbank FinBERT Pipeline"
echo "  Days: $DAYS | Image: $DOCKER_IMAGE"
echo "========================================================"

HF_CACHE="${HF_CACHE:-${HOME}/.cache/huggingface}"
mkdir -p "$HF_CACHE"

DOCKER_RUN="docker run --rm --gpus all"
DOCKER_RUN+=" -v ${PROJECT_DIR}:/workspace"
DOCKER_RUN+=" -v ${HF_CACHE}:/root/.cache/huggingface"
HF_TOKEN="${HF_TOKEN:-${HF_DOWNLOAD_TOKEN:-}}"
if [ -n "${HF_TOKEN}" ]; then
    DOCKER_RUN+=" -e HF_TOKEN=${HF_TOKEN}"
fi
DOCKER_RUN+=" ${DOCKER_IMAGE}"

echo ""
echo "--- Step 1: Collect Sberbank data ---"
$DOCKER_RUN training/data_collection/sber_collect.py --days "$DAYS"

echo ""
echo "--- Step 2: Merge into training set ---"
$DOCKER_RUN training/data_collection/sber_collect.py --merge

echo ""
echo "--- Step 3: Train FinBERT SFT ---"
$DOCKER_RUN training/finbert_sft/train.py

echo ""
echo "--- Step 4: Evaluate ---"
$DOCKER_RUN training/finbert_sft/evaluate.py

echo ""
echo "--- Step 5: Export ONNX ---"
$DOCKER_RUN training/finbert_sft/export_onnx.py

echo ""
echo "========================================================"
echo "  Pipeline complete!"
echo "  Model: models/finbert/"
echo "  ONNX:  models/finbert/model.onnx"
echo "========================================================"
