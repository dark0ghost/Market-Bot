# FinBERT SFT - Model Card

This directory holds the ONNX-exported FinBERT sentiment model used by
`trader-bot` for news sentiment inference (`ml_inference/nlp.rs`).

## Files

| File | Description |
|------|-------------|
| `model.onnx` | ONNX-exported model (3-class sentiment: negative / neutral / positive) |
| `model.json` | Export metadata (base model, labels, max_length, opset) |
| `tokenizer/` | The exact tokenizer used at export time (must match inference) |
| `MANIFEST.json` | SHA-256 checksums of model + tokenizer files |

## Intended use

News headline / short-text financial sentiment classification. Output logits
are reduced to a `sentiment_score` consumed by the decision engine and the
`SentimentGate`.

## How to regenerate

```bash
python training/finbert_sft/train.py       # → training/finbert_sft/<output_dir>/
python training/finbert_sft/export_onnx.py # → models/finbert/model.onnx + tokenizer/
```

`export_onnx.py` writes the tokenizer and a checksum manifest alongside the
ONNX so inference can verify it loads the matching tokenizer.

## Versioning

No automated version field yet - `model.json` records the base model name and
opset. Replace the files in this directory to ship a new version; the hot-reload
watcher in `OrtSessionPool::spawn_watcher` picks up `model.onnx` changes at
runtime.
