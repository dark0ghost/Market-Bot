import io
import zipfile
import requests
from datasets import DatasetDict, Dataset, concatenate_datasets
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

HF_ZIP_URL = "https://huggingface.co/datasets/financial_phrasebank/resolve/main/data/FinancialPhraseBank-v1.0.zip"

CONFIG_TO_FILE = {
    "sentences_allagree": "Sentences_AllAgree.txt",
    "sentences_75agree": "Sentences_75Agree.txt",
    "sentences_66agree": "Sentences_66Agree.txt",
    "sentences_50agree": "Sentences_50Agree.txt",
}


def load_local_parquet() -> DatasetDict | None:
    data_dir = Path("training/finbert_sft/data")
    train_path = data_dir / "train.parquet"
    test_path = data_dir / "test.parquet"
    if not train_path.exists() or not test_path.exists():
        return None
    train_df = pd.read_parquet(train_path)
    test_df = pd.read_parquet(test_path)
    train_df = train_df.rename(columns={"text": "sentence"})
    test_df = test_df.rename(columns={"text": "sentence"})
    if "label" in train_df.columns:
        train_df["label"] = train_df["label"].map(LABEL2ID).astype(int)
        test_df["label"] = test_df["label"].map(LABEL2ID).astype(int)
    train = Dataset.from_pandas(train_df[["sentence", "label"]], preserve_index=False)
    test = Dataset.from_pandas(test_df[["sentence", "label"]], preserve_index=False)
    print(f"Loaded local parquet data ({len(train)} train / {len(test)} test)")
    return DatasetDict({"train": train, "test": test})


def load_financial_phrasebank() -> DatasetDict:
    local = load_local_parquet()
    if local is not None:
        return local

    config_name = CONFIG["training"]["dataset_config"]
    filename = CONFIG_TO_FILE.get(config_name, "Sentences_AllAgree.txt")

    print(f"Downloading FinancialPhraseBank-v1.0 from HuggingFace...")
    resp = requests.get(HF_ZIP_URL, timeout=120)
    resp.raise_for_status()

    with zipfile.ZipFile(io.BytesIO(resp.content)) as z:
        with z.open(f"FinancialPhraseBank-v1.0/{filename}") as f:
            lines = f.read().decode("latin-1").strip().splitlines()
            records = []
            for line in lines:
                parts = line.rsplit("@", 1)
                if len(parts) == 2:
                    records.append((parts[0].strip(), parts[1].strip()))
            df = pd.DataFrame(records, columns=["sentence", "label"])

    df = df[df["label"].isin(LABELS)]
    df["label"] = df["label"].map(LABEL2ID).astype(int)

    print(f"Loaded {len(df)} samples from {filename}")

    dataset = Dataset.from_pandas(df, preserve_index=False)
    dataset = dataset.train_test_split(
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
    combined = combined.drop(columns=["label"])
    combined = combined.rename(columns={"text": "sentence", "label_id": "label"})

    shuffled = combined.sample(frac=1, random_state=42)
    n_train = int(len(shuffled) * CONFIG["training"]["train_test_split"])

    train = Dataset.from_pandas(shuffled.iloc[:n_train][["sentence", "label"]], preserve_index=False)
    test = Dataset.from_pandas(shuffled.iloc[n_train:][["sentence", "label"]], preserve_index=False)

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

    for split in ["train", "test"]:
        if extra is not None:
            base[split] = concatenate_datasets([base[split], extra[split]])

    def fn(batch):
        return tokenize_fn(batch, tokenizer)

    dataset = base.map(fn, batched=True)

    dataset.set_format(
        type="torch",
        columns=["input_ids", "attention_mask", "label"],
    )

    print(f"Final dataset: {len(dataset['train'])} train / {len(dataset['test'])} test")
    return dataset
