#!/bin/bash
set -euo pipefail

echo "=== Step 1: Training FinBERT SFT ==="
python training/finbert_sft/train.py

echo ""
echo "=== Step 2: Evaluation ==="
python training/finbert_sft/evaluate.py

echo ""
echo "=== Step 3: Export to ONNX ==="
python training/finbert_sft/export_onnx.py

echo ""
echo "=== Pipeline complete ==="
ls -la models/finbert/