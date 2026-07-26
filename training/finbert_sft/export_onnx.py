import yaml
import logging
import torch
from pathlib import Path
from transformers import AutoModelForSequenceClassification, AutoTokenizer

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

with open("training/finbert_sft/config.yaml") as f:
    CONFIG = yaml.safe_load(f)

OUTPUT_DIR = CONFIG["training"]["output_dir"]
ONNX_PATH = CONFIG["onnx"]["output_path"]
OPSET = CONFIG["onnx"]["opset"]
MAX_LENGTH = CONFIG["model"]["max_length"]
LABELS = CONFIG["model"]["labels"]


def export_to_onnx():
    logger.info(f"Loading fine-tuned model from {OUTPUT_DIR}...")
    model = AutoModelForSequenceClassification.from_pretrained(OUTPUT_DIR)
    tokenizer = AutoTokenizer.from_pretrained(OUTPUT_DIR)
    model.eval()

    dummy_input = tokenizer(
        "Sample financial text for ONNX export.",
        padding="max_length",
        truncation=True,
        max_length=MAX_LENGTH,
        return_tensors="pt",
    )

    Path(ONNX_PATH).parent.mkdir(parents=True, exist_ok=True)

    with torch.no_grad():
        torch.onnx.export(
            model,
            args=(
                dummy_input["input_ids"],
                dummy_input["attention_mask"],
            ),
            f=ONNX_PATH,
            input_names=["input_ids", "attention_mask"],
            output_names=["logits"],
            dynamic_axes={
                "input_ids": {0: "batch_size", 1: "sequence_length"},
                "attention_mask": {0: "batch_size", 1: "sequence_length"},
                "logits": {0: "batch_size"},
            },
            opset_version=OPSET,
            do_constant_folding=True,
        )

    logger.info(f"ONNX model exported to {ONNX_PATH}")

    import json
    metadata = {
        "model": CONFIG["model"]["name"],
        "labels": LABELS,
        "max_length": MAX_LENGTH,
        "opset": OPSET,
    }
    meta_path = Path(ONNX_PATH).with_suffix(".json")
    with open(meta_path, "w") as f:
        json.dump(metadata, f, indent=2)
    logger.info(f"Metadata saved to {meta_path}")

    import onnx
    onnx_model = onnx.load(ONNX_PATH)
    onnx.checker.check_model(onnx_model)
    logger.info("ONNX model validation passed.")

    return str(ONNX_PATH)


def main():
    export_to_onnx()


if __name__ == "__main__":
    main()
