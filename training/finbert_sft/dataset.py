from datasets import load_dataset, DatasetDict, Dataset, concatenate_datasets
from transformers import AutoTokenizer
import yaml
import pandas as pd
from pathlib import Path
from typing import Dict, Optional

with open("training/finbert_sft/config.yaml") as f:
    CONFIG = yaml.safe_load(f)

MODEL_NAME = CONFIG["model"]["name"]
MAX_LENGTH = CONFIG["model"]["max_length"]
LABELS = CONFIG["model"]["labels"]
LABEL2ID = {l: i for i, l in enumerate(LABELS)}
ID2LABEL = {i: l for i, l in enumerate(LABELS)}


def load_financial_phrasebank() -> DatasetDict:
    dataset = load_dataset(
        CONFIG["training"]["dataset"],
        CONFIG["training"]["dataset_config"],
    )
    dataset = dataset["train"].train_test_split(
        test_size=1 - CONFIG["training"]["train_test_split"],
        seed=CONFIG["training"]["seed"],
    )
    return DatasetDict({
        "train": dataset["train"],
        "test": dataset["test"],
    })


def load_collected_data() -> DatasetDict:
    data_dir = Path("training/data_collected")
    if not data_dir.exists():
        return None

    parquets = list(data_dir.glob("*.parquet"))
    if not parquets:
        return None

    dfs = []
    for p in parquets:
        df = pd.read_parquet(p)
        df = df[df["label"].notna()]
        if not df.empty:
            dfs.append(df)

    if not dfs:
        return None

    combined = pd.concat(dfs, ignore_index=True)
    combined = combined.drop_duplicates(subset=["text_hash"])
    combined = combined.rename(columns={"text": "sentence", "label_id": "label"})
    combined["label"] = combined["label"].astype(int)

    shuffled = combined.sample(frac=1, random_state=42)
    n_train = int(len(shuffled) * CONFIG["training"]["train_test_split"])

    train = Dataset.from_pandas(shuffled.iloc[:n_train], preserve_index=False)
    test = Dataset.from_pandas(shuffled.iloc[n_train:], preserve_index=False)

    print(f"Loaded {len(combined)} collected samples ({len(train)} train / {len(test)} test)")
    return DatasetDict({"train": train, "test": test})


def tokenize_fn(batch: Dict, tokenizer: AutoTokenizer) -> Dict:
    return tokenizer(
        batch["sentence"],
        padding="max_length",
        truncation=True,
        max_length=MAX_LENGTH,
    )


def prepare_dataset(tokenizer: AutoTokenizer) -> DatasetDict:
    base = load_financial_phrasebank()
    extra = load_collected_data()

    if extra is not None:
        for split in ["train", "test"]:
            base[split] = concatenate_datasets([base[split], extra[split]])

    def fn(batch):
        return tokenize_fn(batch, tokenizer)

    dataset = base.map(fn, batched=True)

    def map_label(batch):
        if isinstance(batch["label"], str):
            return {"label": LABEL2ID.get(batch["label"], 1)}
        return {"label": batch["label"]}

    dataset = dataset.map(map_label)

    dataset.set_format(
        type="torch",
        columns=["input_ids", "attention_mask", "label"],
    )

    print(f"Final dataset: {len(dataset['train'])} train / {len(dataset['test'])} test")
    return dataset
