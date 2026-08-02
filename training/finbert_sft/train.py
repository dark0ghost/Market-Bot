import torch
import yaml
import logging
from transformers import (
    TrainingArguments,
    Trainer,
    EarlyStoppingCallback,
)
from dataset import prepare_dataset
from model import load_model, load_tokenizer, save_model_and_tokenizer

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

with open("training/finbert_sft/config.yaml") as f:
    CONFIG = yaml.safe_load(f)


def compute_metrics(eval_pred):
    from sklearn.metrics import (
        accuracy_score,
        cohen_kappa_score,
        precision_recall_fscore_support,
    )
    import numpy as np

    logits, labels = eval_pred
    predictions = np.argmax(logits, axis=-1)
    precision, recall, f1, _ = precision_recall_fscore_support(
        labels, predictions, average="weighted"
    )
    acc = accuracy_score(labels, predictions)
    kappa = cohen_kappa_score(labels, predictions)

    metrics = {
        "accuracy": acc,
        "f1": f1,
        "precision": precision,
        "recall": recall,
        "cohen_kappa": kappa,
    }

    # Expected Calibration Error (ECE) - the TradingAgent uses confidence scores,
    # so calibration matters. Bucket softmax confidences into equal-width bins and
    # measure the gap between confidence and accuracy per bin.
    try:
        probs = _softmax(logits)
        confidences = np.max(probs, axis=-1)
        n_bins = 10
        bin_edges = np.linspace(0.0, 1.0, n_bins + 1)
        ece = 0.0
        n = len(labels)
        for i in range(n_bins):
            lo, hi = bin_edges[i], bin_edges[i + 1]
            mask = (confidences > lo) & (confidences <= hi)
            if mask.sum() == 0:
                continue
            bin_acc = (predictions[mask] == labels[mask]).mean()
            bin_conf = confidences[mask].mean()
            ece += (mask.sum() / n) * abs(bin_conf - bin_acc)
        metrics["ece"] = float(ece)
    except Exception as e:  # calibration is diagnostic, never fatal
        logger.warning(f"ECE computation skipped: {e}")

    return metrics


def _softmax(x):
    import numpy as np

    x = np.asarray(x, dtype=np.float64)
    x = x - x.max(axis=-1, keepdims=True)
    e = np.exp(x)
    return e / e.sum(axis=-1, keepdims=True)


def train():
    tokenizer = load_tokenizer()
    model = load_model()
    dataset = prepare_dataset(tokenizer)

    training_args = TrainingArguments(
        output_dir=CONFIG["training"]["output_dir"],
        eval_strategy="steps",
        eval_steps=CONFIG["training"]["eval_steps"],
        save_steps=CONFIG["training"]["save_steps"],
        logging_steps=CONFIG["training"]["logging_steps"],
        per_device_train_batch_size=CONFIG["training"]["batch_size"],
        per_device_eval_batch_size=CONFIG["training"]["batch_size"],
        num_train_epochs=CONFIG["training"]["epochs"],
        learning_rate=CONFIG["training"]["learning_rate"],
        weight_decay=CONFIG["training"]["weight_decay"],
        warmup_ratio=CONFIG["training"]["warmup_ratio"],
        load_best_model_at_end=True,
        metric_for_best_model="f1",
        greater_is_better=True,
        report_to="none",
        fp16=torch.cuda.is_available(),
        dataloader_num_workers=0,
        remove_unused_columns=False,
        seed=CONFIG["training"]["seed"],
    )

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=dataset["train"],
        eval_dataset=dataset["test"],
        processing_class=tokenizer,
        compute_metrics=compute_metrics,
        callbacks=[EarlyStoppingCallback(early_stopping_patience=3)],
    )

    logger.info("Starting training...")
    trainer.train()
    logger.info("Training complete.")

    eval_results = trainer.evaluate()
    logger.info(f"Eval results: {eval_results}")

    save_model_and_tokenizer(model, tokenizer)
    logger.info(f"Model saved to {CONFIG['training']['output_dir']}")

    return eval_results


def main():
    train()


if __name__ == "__main__":
    main()
