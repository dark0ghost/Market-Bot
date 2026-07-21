from transformers import AutoModelForSequenceClassification, AutoTokenizer
import yaml
from pathlib import Path

with open("training/finbert_sft/config.yaml") as f:
    CONFIG = yaml.safe_load(f)

MODEL_NAME = CONFIG["model"]["name"]
NUM_LABELS = CONFIG["model"]["num_labels"]


def load_model() -> AutoModelForSequenceClassification:
    model = AutoModelForSequenceClassification.from_pretrained(
        MODEL_NAME,
        num_labels=NUM_LABELS,
        ignore_mismatched_sizes=True,
    )
    return model


def load_tokenizer() -> AutoTokenizer:
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    return tokenizer


def save_model_and_tokenizer(
    model: AutoModelForSequenceClassification,
    tokenizer: AutoTokenizer,
    path: str = CONFIG["training"]["output_dir"],
):
    Path(path).mkdir(parents=True, exist_ok=True)
    model.save_pretrained(path)
    tokenizer.save_pretrained(path)

    import json
    with open(f"{path}/labels.json", "w") as f:
        json.dump(CONFIG["model"]["labels"], f)
