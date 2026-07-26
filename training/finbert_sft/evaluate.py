import yaml
import logging
import torch
import numpy as np
from datasets import DatasetDict
from transformers import AutoModelForSequenceClassification, AutoTokenizer
from sklearn.metrics import (
    accuracy_score,
    precision_recall_fscore_support,
    confusion_matrix,
    classification_report,
)
from dataset import prepare_dataset

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

with open("training/finbert_sft/config.yaml") as f:
    CONFIG = yaml.safe_load(f)

LABELS = CONFIG["model"]["labels"]
OUTPUT_DIR = CONFIG["training"]["output_dir"]


def evaluate():
    logger.info(f"Loading model from {OUTPUT_DIR}...")
    model = AutoModelForSequenceClassification.from_pretrained(OUTPUT_DIR)
    tokenizer = AutoTokenizer.from_pretrained(OUTPUT_DIR)
    model.eval()

    dataset = prepare_dataset(tokenizer)

    all_preds = []
    all_labels = []

    with torch.no_grad():
        for batch in torch.utils.data.DataLoader(
            dataset["test"], batch_size=CONFIG["training"]["batch_size"]
        ):
            outputs = model(
                input_ids=batch["input_ids"],
                attention_mask=batch["attention_mask"],
            )
            preds = torch.argmax(outputs.logits, dim=-1)
            all_preds.extend(preds.numpy())
            all_labels.extend(batch["label"].numpy())

    acc = accuracy_score(all_labels, all_preds)
    precision, recall, f1, _ = precision_recall_fscore_support(
        all_labels, all_preds, average="weighted"
    )

    logger.info(f"Test Accuracy: {acc:.4f}")
    logger.info(f"Weighted Precision: {precision:.4f}")
    logger.info(f"Weighted Recall: {recall:.4f}")
    logger.info(f"Weighted F1: {f1:.4f}")

    report = classification_report(
        all_labels, all_preds,
        target_names=LABELS,
        labels=list(range(len(LABELS))),
        digits=4,
        zero_division=0,
    )
    logger.info(f"\nClassification Report:\n{report}")

    cm = confusion_matrix(all_labels, all_preds)
    logger.info(f"Confusion Matrix:\n{cm}")

    return {"accuracy": acc, "f1": f1, "precision": precision, "recall": recall}


def main():
    evaluate()


if __name__ == "__main__":
    main()
