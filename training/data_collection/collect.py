"""
Data collection pipeline for FinBERT SFT.
Sources: RSS feeds + Perplexica → Ollama labeling → Parquet.

Usage:
    python training/data_collection/collect.py                  # single run
    python training/data_collection/collect.py --watch          # loop every N min
    python training/data_collection/collect.py --merge          # merge into training set

Output: training/data_collected/*.parquet
"""

import argparse
import hashlib
import json
import logging
import re
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Optional
from urllib.parse import urlparse

import pandas as pd
import requests
import yaml

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("data_collector")

HERE = Path(__file__).parent
with open(HERE / "config.yaml") as f:
    CONFIG = yaml.safe_load(f)

OUTPUT_DIR = Path(CONFIG["output"]["dir"])
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# ─── Data Schema ──────────────────────────────────────────────────────

SENTIMENT_MAP = {
    "positive": 2,
    "neutral": 1,
    "negative": 0,
}


@dataclass
class Sample:
    text: str
    source: str
    url: str = ""
    title: str = ""
    published: str = ""
    label: Optional[str] = None
    label_confidence: float = 0.0
    label_method: str = ""
    collected_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    @property
    def text_hash(self) -> str:
        return hashlib.sha256(self.text.encode()).hexdigest()[:16]

    def to_row(self) -> dict:
        d = asdict(self)
        d["text_hash"] = self.text_hash
        d["label_id"] = SENTIMENT_MAP.get(self.label, 1)
        return d


# ─── Deduplication ────────────────────────────────────────────────────


class DedupStore:
    def __init__(self, path: Path):
        self.path = path
        self.seen: set[str] = set()
        if path.exists():
            df = pd.read_parquet(path)
            if "text_hash" in df.columns:
                self.seen = set(df["text_hash"].dropna().unique())
            logger.info(f"Loaded {len(self.seen)} existing samples from {path}")

    def is_new(self, sample: Sample) -> bool:
        return sample.text_hash not in self.seen

    def append(self, sample: Sample):
        self.seen.add(sample.text_hash)


# ─── RSS Collector ────────────────────────────────────────────────────


def fetch_rss(url: str, timeout: int) -> list[dict]:
    headers = {"User-Agent": CONFIG["collection"]["user_agent"]}
    try:
        resp = requests.get(url, headers=headers, timeout=timeout)
        resp.raise_for_status()
    except requests.RequestException as e:
        logger.warning(f"RSS fetch failed: {url} — {e}")
        return []

    items = []
    # Malformed/invalid XML must not crash the whole collection run — skip the feed.
    try:
        root = ET.fromstring(resp.content)
    except ET.ParseError as e:
        logger.warning(f"RSS parse failed (malformed XML): {url} — {e}")
        return []
    for item in root.iter("item"):
        title = (item.findtext("title") or "").strip()
        desc = (item.findtext("description") or "").strip()
        text = f"{title}. {desc}".strip()
        if len(text) < 20:
            continue
        pub_date = item.findtext("pubDate") or ""
        link = item.findtext("link") or ""
        items.append({"text": cleanup(text), "title": title, "url": link, "published": pub_date})
    return items


def cleanup(text: str) -> str:
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text[:512]


# ─── Perplexica Collector ─────────────────────────────────────────────


def fetch_perplexica(topic: str, base_url: str, timeout: int) -> Optional[str]:
    try:
        resp = requests.post(
            f"{base_url}/api/search",
            json={"query": topic, "focusMode": "web"},
            timeout=timeout,
        )
        resp.raise_for_status()
        data = resp.json()
        raw = data.get("message", data.get("answer", data.get("sources", "")))
        if isinstance(raw, list):
            raw = " ".join(s.get("content", "") for s in raw)
        return cleanup(str(raw))
    except requests.RequestException as e:
        logger.warning(f"Perplexica failed for '{topic}': {e}")
        return None


# ─── Ollama Labeler ───────────────────────────────────────────────────


def label_sentiment(text: str, label_retries: int = 3) -> tuple[Optional[str], float]:
    prompt = f"""Определи финансовую тональность текста. Ответь строго JSON: {{"label": "positive|neutral|negative", "confidence": 0.0-1.0}}

Текст: {text}"""

    for attempt in range(label_retries):
        try:
            resp = requests.post(
                f"{CONFIG['ollama']['base_url']}/api/generate",
                json={
                    "model": CONFIG["ollama"]["model"],
                    "prompt": prompt,
                    "temperature": CONFIG["ollama"]["temperature"],
                    "stream": False,
                },
                timeout=30,
            )
            resp.raise_for_status()
            raw = resp.json()["response"].strip()
            raw = raw.removeprefix("```json").removeprefix("```").removesuffix("```").strip()
            parsed = json.loads(raw)
            label = parsed.get("label", "neutral")
            confidence = float(parsed.get("confidence", 0.5))
            if label not in SENTIMENT_MAP:
                label = "neutral"
            return label, min(confidence, 1.0)
        except Exception as e:
            logger.warning(f"Ollama label attempt {attempt + 1} failed: {e}")
            time.sleep(1)
    return None, 0.0


# ─── Sources ──────────────────────────────────────────────────────────


def collect_from_rss(dedup: DedupStore, max_samples: int) -> list[Sample]:
    samples = []
    # Be polite between feeds to avoid hammering sources / getting rate-limited.
    inter_feed_delay = CONFIG["collection"].get("inter_feed_delay_sec", 2)
    for feed in CONFIG["rss_feeds"]:
        logger.info(f"Fetching RSS: {feed['name']}")
        items = fetch_rss(feed["url"], CONFIG["collection"]["request_timeout"])
        for item in items:
            if len(samples) >= max_samples:
                break
            sample = Sample(text=item["text"], source=f"rss:{feed['name']}", **item)
            if dedup.is_new(sample):
                samples.append(sample)
        logger.info(f"  → {len(items)} items from {feed['name']}")
        time.sleep(inter_feed_delay)
    return samples


def collect_from_perplexica(dedup: DedupStore, max_samples: int) -> list[Sample]:
    samples = []
    cfg = CONFIG["perplexica"]
    for topic in cfg["topics"]:
        if len(samples) >= max_samples:
            break
        logger.info(f"Perplexica search: {topic}")
        text = fetch_perplexica(topic, cfg["base_url"], CONFIG["collection"]["request_timeout"])
        if text and len(text) > 20:
            sample = Sample(text=text, source="perplexica", title=topic)
            if dedup.is_new(sample):
                samples.append(sample)
    return samples


# ─── Labeling ─────────────────────────────────────────────────────────


def label_samples(samples: list[Sample]) -> list[Sample]:
    labeled = []
    for i, sample in enumerate(samples):
        label, conf = label_sentiment(sample.text)
        if label:
            sample.label = label
            sample.label_confidence = conf
            sample.label_method = "ollama"
            labeled.append(sample)
        if (i + 1) % 10 == 0:
            logger.info(f"Labeled {i + 1}/{len(samples)}")
    return labeled


# ─── Storage ──────────────────────────────────────────────────────────


def save_samples(samples: list[Sample], dedup: DedupStore):
    if not samples:
        logger.info("No new samples to save")
        return

    rows = [s.to_row() for s in samples]

    existing = None
    dedup_path = dedup.path
    if dedup_path.exists():
        existing = pd.read_parquet(dedup_path)

    new_df = pd.DataFrame(rows)
    combined = pd.concat([existing, new_df], ignore_index=True) if existing is not None else new_df
    combined = combined.drop_duplicates(subset=["text_hash"])

    combined.to_parquet(dedup_path, index=False)
    logger.info(f"Saved {len(new_df)} new samples → {dedup_path} (total: {len(combined)})")

    for s in samples:
        dedup.append(s)


# ─── Merge into training set ──────────────────────────────────────────


def merge_into_training():
    src = OUTPUT_DIR
    dst = HERE.parent / "finbert_sft" / "data"
    dst.mkdir(parents=True, exist_ok=True)

    parquet_files = list(src.glob("*.parquet"))
    if not parquet_files:
        logger.warning(f"No parquet files found in {src}")
        return

    dfs = [pd.read_parquet(p) for p in parquet_files]
    combined = pd.concat(dfs, ignore_index=True)
    combined = combined.drop_duplicates(subset=["text_hash"])
    combined = combined[combined["label"].notna()]

    train_ratio = CONFIG["output"]["train_ratio"]
    shuffled = combined.sample(frac=1, random_state=42)
    n_train = int(len(shuffled) * train_ratio)

    train = shuffled.iloc[:n_train]
    test = shuffled.iloc[n_train:]

    train.to_parquet(dst / "train.parquet", index=False)
    test.to_parquet(dst / "test.parquet", index=False)

    label_dist = combined["label"].value_counts()
    logger.info(f"Merged: {len(combined)} samples ({len(train)} train / {len(test)} test)")
    logger.info(f"Label distribution:\n{label_dist}")

    return combined


# ─── Main ─────────────────────────────────────────────────────────────


def run_collection(max_samples: Optional[int] = None):
    dedup_path = OUTPUT_DIR / "dataset.parquet"
    dedup = DedupStore(dedup_path)
    max_s = max_samples or CONFIG["output"]["max_samples_per_source"]

    samples = []
    samples += collect_from_rss(dedup, max_s)
    samples += collect_from_perplexica(dedup, max_s)

    if not samples:
        logger.info("No new samples to label")
        return

    logger.info(f"Labeling {len(samples)} samples with Ollama ({CONFIG['ollama']['model']})...")
    labeled = label_samples(samples)
    logger.info(f"Successfully labeled: {len(labeled)}/{len(samples)}")

    save_samples(labeled, dedup)


def watch_loop():
    interval = CONFIG["collection"]["interval_minutes"]
    logger.info(f"Starting watch loop every {interval} min")
    while True:
        run_collection()
        logger.info(f"Sleeping {interval} min...")
        time.sleep(interval * 60)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--watch", action="store_true", help="Run in watch loop")
    parser.add_argument("--merge", action="store_true", help="Merge collected data into training set")
    parser.add_argument("--max-samples", type=int, default=None, help="Max samples per source")
    args = parser.parse_args()

    if args.merge:
        merge_into_training()
    elif args.watch:
        watch_loop()
    else:
        run_collection(args.max_samples)


if __name__ == "__main__":
    main()
