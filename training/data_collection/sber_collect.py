"""
Universal MOEX data collector - candles + RSS + Perplexica → labeled training set.

Usage:
    # Collect for T-Technologies, last 180 days
    python training/data_collection/sber_collect.py --ticker T --days 180

    # Collect for Sberbank, last 30 days, custom window
    python training/data_collection/sber_collect.py --ticker SBER --days 30 --window 30

    # Merge all collected parquets into train/test
    python training/data_collection/sber_collect.py --merge

    # Add keywords for a new ticker
    python training/data_collection/sber_collect.py --ticker YDEX --keywords yandex яндекс
"""

import argparse
import hashlib
import logging
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Optional

import pandas as pd
import requests
import yaml

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[logging.StreamHandler()],
)
logger = logging.getLogger("collector")

HERE = Path(__file__).parent
with open(HERE / "config.yaml") as f:
    CONFIG = yaml.safe_load(f)

OUTPUT_DIR = HERE.parent / "data_collected"
try:
    test = OUTPUT_DIR / ".write_test"
    test.touch(exist_ok=True)
    test.unlink()
except PermissionError:
    OUTPUT_DIR = HERE.parent / "finbert_sft" / "data"
    logger.info(f"data_collected not writable, using {OUTPUT_DIR}")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

LABEL_MAP = {"positive": 2, "neutral": 1, "negative": 0}

TICKER_CONFIG: dict[str, dict] = {
    "SBER": {
        "keywords": ["сбербанк", "сбер", "sber", "sberbank"],
        "topics": ["Сбербанк акции", "Сбер финансовые результаты", "Sberbank stock news"],
    },
    "T": {
        "keywords": ["т-технологии", "т-тех", "t-technologies", "t-t", "tcs", "tcsg",
                     "тинькофф", "тинька", "tinkoff", "tcs group", "t-group"],
        "topics": ["Т-Технологии акции", "Тинькофф результаты", "Tinkoff TCSG news"],
    },
}


@dataclass
class Candle:
    open: float
    close: float
    high: float
    low: float
    value: float
    volume: int
    begin: datetime


@dataclass
class NewsSample:
    text: str
    source: str
    ticker: str = ""
    url: str = ""
    title: str = ""
    published: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    label: Optional[str] = None
    price_change: float = 0.0
    volume_ratio: float = 0.0

    @property
    def text_hash(self) -> str:
        return hashlib.sha256(self.text.encode()).hexdigest()[:16]

    def to_row(self) -> dict:
        return {
            "text": self.text,
            "source": self.source,
            "ticker": self.ticker,
            "url": self.url,
            "title": self.title,
            "published": self.published.isoformat(),
            "label": self.label or "neutral",
            "label_id": LABEL_MAP.get(self.label, 1),
            "price_change": self.price_change,
            "volume_ratio": self.volume_ratio,
            "text_hash": self.text_hash,
        }


# ─── MOEX ──────────────────────────────────────────────────────────

MOEX_BOARD = "TQBR"


def fetch_candles(ticker: str, from_date: str, till_date: str) -> list[Candle]:
    url = (f"https://iss.moex.com/iss/engines/stock/markets/shares/boards/{MOEX_BOARD}"
           f"/securities/{ticker}/candles.json")
    all_candles: list[Candle] = []
    from_dt = datetime.strptime(from_date, "%Y-%m-%d")
    till_dt = datetime.strptime(till_date, "%Y-%m-%d")
    current = from_dt
    while current <= till_dt:
        day_str = current.strftime("%Y-%m-%d")
        params = {"from": day_str, "till": day_str, "interval": "1", "limit": 50000}
        try:
            resp = requests.get(url, params=params, timeout=30)
            resp.raise_for_status()
            data = resp.json()
        except Exception as e:
            logger.warning(f"MOEX fetch failed for {day_str}: {e}")
            current += timedelta(days=1)
            continue

        rows = data.get("candles", {}).get("data", [])
        if not rows:
            current += timedelta(days=1)
            continue

        candles = []
        for r in rows:
            try:
                candles.append(Candle(
                    open=float(r[0]), close=float(r[1]), high=float(r[2]), low=float(r[3]),
                    value=float(r[4]), volume=int(r[5]),
                    begin=datetime.fromisoformat(r[6]),
                ))
            except (ValueError, IndexError):
                continue

        all_candles.extend(candles)
        logger.info(f"Fetched {len(candles)} candles for {day_str} (total {len(all_candles)})")
        current += timedelta(days=1)

    return all_candles


# ─── RSS ───────────────────────────────────────────────────────────

def fetch_rss(url: str, timeout: int, keywords: list[str]) -> list[dict]:
    headers = {"User-Agent": CONFIG["collection"]["user_agent"]}
    try:
        resp = requests.get(url, headers=headers, timeout=timeout)
        resp.raise_for_status()
    except requests.RequestException as e:
        logger.warning(f"RSS fetch failed: {url} - {e}")
        return []

    items = []
    root = ET.fromstring(resp.content)
    for item in root.iter("item"):
        title = (item.findtext("title") or "").strip()
        desc = (item.findtext("description") or "").strip()
        text = f"{title}. {desc}".strip()
        if len(text) < 20:
            continue
        text_lower = text.lower()
        if not any(
            re.search(rf'\b{re.escape(kw)}\b', text_lower) if len(kw) <= 3
            else kw in text_lower
            for kw in keywords
        ):
            continue
        pub_date = item.findtext("pubDate") or ""
        link = item.findtext("link") or ""
        items.append({
            "text": _cleanup(text), "title": title,
            "url": link, "published": _parse_rss_date(pub_date),
        })
    return items


def _cleanup(text: str) -> str:
    text = re.sub(r"<[^>]+>", " ", text)
    text = re.sub(r"\s+", " ", text).strip()
    return text[:512]


def _parse_rss_date(date_str: str) -> Optional[datetime]:
    for fmt in [
        "%a, %d %b %Y %H:%M:%S %z",
        "%a, %d %b %Y %H:%M:%S %Z",
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S",
    ]:
        try:
            return datetime.strptime(date_str, fmt)
        except ValueError:
            continue
    return None


# ─── SearXNG (bundled with Perplexica) ──────────────────────────────

def fetch_searxng(timeout: int, topics: list[str]) -> list[dict]:
    cfg = CONFIG["searxng"]
    items = []
    for topic in topics:
        try:
            resp = requests.get(
                f"{cfg['base_url']}/search",
                params={"q": topic, "format": "json", "language": cfg["language"]},
                timeout=timeout,
            )
            resp.raise_for_status()
            data = resp.json()
            for r in data.get("results", [])[:cfg.get("max_results", 20)]:
                title = (r.get("title") or "").strip()
                content = (r.get("content") or "").strip()
                if not title and not content:
                    continue
                text = _cleanup(f"{title}. {content}" if title else content)
                if len(text) < 20:
                    continue
                pub = r.get("publishedDate")
                if pub:
                    try:
                        published = datetime.fromisoformat(pub.replace("Z", "+00:00"))
                    except ValueError:
                        published = datetime.now(timezone.utc)
                else:
                    published = datetime.now(timezone.utc)
                items.append({
                    "text": text, "source": "searxng",
                    "title": title, "url": r.get("url", ""),
                    "published": published,
                })
        except Exception as e:
            logger.warning(f"SearXNG failed for '{topic}': {e}")
    return items

# ─── Direct web search via Google News RSS (no API key needed) ─────

def fetch_direct_news(timeout: int, topics: list[str]) -> list[dict]:
    items = []
    seen_urls = set()
    sources = [
        ("https://news.yandex.ru/finance.rss", "yandex_news"),
        ("https://lenta.ru/rss/news", "lenta"),
        ("https://www.mk.ru/rss/news/index.xml", "mk"),
    ]
    for url, name in sources:
        try:
            resp = requests.get(
                url,
                headers={"User-Agent": CONFIG["collection"]["user_agent"]},
                timeout=min(timeout, 10),
            )
            resp.raise_for_status()
            root = ET.fromstring(resp.content)
            for item in root.iter("item"):
                title = (item.findtext("title") or "").strip()
                desc = (item.findtext("description") or "").strip()
                link = (item.findtext("link") or "").strip()
                pub = (item.findtext("pubDate") or "").strip()
                if not title or len(title) < 10:
                    continue
                text = _cleanup(f"{title}. {desc}" if desc else title)
                text_lower = text.lower()
                kw_match = any(kw in text_lower for kw in
                    ["тиньк", "т-техно", "tcs", "tinkoff", "т-тех", "t-techn"])
                if not kw_match:
                    continue
                if link in seen_urls:
                    continue
                seen_urls.add(link)
                if len(text) < 20:
                    continue
                published = _parse_rss_date(pub) or datetime.now(timezone.utc)
                items.append({
                    "text": text, "source": name,
                    "title": title, "url": link,
                    "published": published,
                })
        except Exception as e:
            logger.debug(f"Direct news {name} failed: {e}")
    if items:
        logger.info(f"Direct news: {len(items)} items from RSS aggregation")
    return items


# ─── WebSearch cache (pre-collected via IDE web_search tool) ───────

def fetch_websearch_cache(ticker: str) -> list[dict]:
    path = HERE / f"{ticker}_news_websearch.json"
    if not path.exists():
        return []
    try:
        import json
        with open(path) as f:
            raw = json.load(f)
        items = []
        for r in raw:
            text = _cleanup(r.get("text", ""))
            if len(text) < 20:
                continue
            pub_str = r.get("published", "")
            if pub_str:
                try:
                    published = datetime.fromisoformat(pub_str.replace("Z", "+00:00"))
                except ValueError:
                    published = datetime.now(timezone.utc)
            else:
                published = datetime.now(timezone.utc)
            items.append({
                "text": text,
                "source": r.get("source", "websearch"),
                "title": r.get("title", ""),
                "url": r.get("url", ""),
                "published": published,
            })
        logger.info(f"Websearch cache: {len(items)} items for {ticker}")
        return items
    except Exception as e:
        logger.warning(f"Websearch cache failed: {e}")
        return []


# ─── Fallback: Perplexica API (requires UI setup) ──────────────────

def fetch_perplexica(timeout: int, topics: list[str]) -> list[dict]:
    try:
        cfg = CONFIG["perplexica"]
        items = []
        for topic in topics:
            try:
                resp = requests.post(
                    f"{cfg['base_url']}/api/search",
                    json={"query": topic, "focusMode": "web"},
                    timeout=timeout,
                )
                resp.raise_for_status()
                data = resp.json()
                raw = data.get("message", data.get("answer", data.get("sources", "")))
                if isinstance(raw, list):
                    raw = " ".join(s.get("content", "") for s in raw)
                text = _cleanup(str(raw))
                if len(text) > 20:
                    items.append({
                        "text": text, "source": "perplexica",
                        "title": topic, "published": datetime.now(timezone.utc),
                    })
            except Exception as e:
                logger.warning(f"Perplexica failed for '{topic}': {e}")
        return items
    except Exception:
        return []


# ─── Labeling ──────────────────────────────────────────────────────

def label_by_price(
    news_items: list[dict],
    candles: list[Candle],
    window_min: int = 60,
    threshold: float = 0.001,
    vol_mult: float = 1.2,
    ticker: str = "",
) -> list[NewsSample]:
    if not candles:
        logger.warning("No candles for labeling")
        return []

    candles_sorted = sorted(candles, key=lambda c: c.begin)
    avg_volume = sum(c.volume for c in candles_sorted) / max(len(candles_sorted), 1)

    samples: list[NewsSample] = []
    matched = 0
    for item in news_items:
        pub_time = item.get("published")
        if not pub_time or not isinstance(pub_time, datetime):
            continue

        if pub_time.tzinfo is None:
            pub_time = pub_time.replace(tzinfo=timezone.utc)

        best_idx = None
        for i, c in enumerate(candles_sorted):
            if c.begin.tzinfo is None:
                c_begin = c.begin.replace(tzinfo=timezone.utc)
            else:
                c_begin = c.begin
            if c_begin >= pub_time:
                best_idx = i
                break

        if best_idx is None or best_idx >= len(candles_sorted) - 2:
            continue

        matched += 1
        open_candle = candles_sorted[best_idx]
        lookahead = min(best_idx + window_min, len(candles_sorted) - 1)
        close_candle = candles_sorted[lookahead]
        price_change = (close_candle.close - open_candle.open) / open_candle.open if open_candle.open else 0.0

        window_volume = sum(c.volume for c in candles_sorted[best_idx:lookahead + 1])
        window_avg = window_volume / max(lookahead - best_idx + 1, 1)
        volume_ratio = window_avg / max(avg_volume, 1)

        if price_change > threshold and volume_ratio > vol_mult:
            label = "positive"
        elif price_change < -threshold and volume_ratio > vol_mult:
            label = "negative"
        else:
            label = "neutral"

        samples.append(NewsSample(
            text=item["text"], source=item.get("source", "rss"),
            ticker=ticker,
            url=item.get("url", ""), title=item.get("title", ""),
            published=pub_time, label=label,
            price_change=price_change, volume_ratio=volume_ratio,
        ))

    logger.info(f"Matched {matched}/{len(news_items)} → {sum(1 for s in samples if s.label!='neutral')} non-neutral")
    return samples


# ─── Main ──────────────────────────────────────────────────────────

def run(
    ticker: str,
    days: int = 7,
    window: int = 60,
    threshold: float = 0.001,
    vol_mult: float = 1.2,
    keywords: Optional[list[str]] = None,
    topics: Optional[list[str]] = None,
):
    till = datetime.now(timezone.utc)
    candle_days = max(days * 2, 30)
    from_date = (till - timedelta(days=candle_days)).strftime("%Y-%m-%d")
    till_str = till.strftime("%Y-%m-%d")

    cfg = TICKER_CONFIG.get(ticker, {})
    keywords = keywords or cfg.get("keywords", [ticker.lower()])
    topics = topics or cfg.get("topics", [f"{ticker} stock news"])

    logger.info(f"Fetching {ticker} candles {from_date} → {till_str}")
    candles = fetch_candles(ticker, from_date, till_str)
    logger.info(f"Total candles: {len(candles)}")

    news_items = []
    for feed in CONFIG["rss_feeds"]:
        logger.info(f"RSS: {feed['name']}")
        items = fetch_rss(feed["url"], CONFIG["collection"]["request_timeout"], keywords)
        for it in items:
            it["source"] = f"rss:{feed['name']}"
        news_items.extend(items)
        logger.info(f"  → {len(items)} items")

    ticker_topics = topics
    if ticker in CONFIG.get("perplexica", {}).get("ticker_topics", {}):
        ticker_topics = CONFIG["perplexica"]["ticker_topics"][ticker]
        logger.info(f"Using ticker-specific topics: {ticker_topics}")

    logger.info("Websearch cache...")
    news_items.extend(fetch_websearch_cache(ticker))

    logger.info("Google News RSS search...")
    news_items.extend(fetch_direct_news(CONFIG["collection"]["request_timeout"], ticker_topics))

    logger.info("SearXNG web search...")
    news_items.extend(fetch_searxng(CONFIG["collection"]["request_timeout"], ticker_topics))

    logger.info("Perplexica (fallback)...")
    news_items.extend(fetch_perplexica(CONFIG["collection"]["request_timeout"], ticker_topics))

    logger.info(f"Total news: {len(news_items)}")
    if not news_items:
        logger.warning("No news found")
        return

    if candles:
        first_candle = min(c.begin for c in candles)
        if first_candle.tzinfo is not None:
            first_candle = first_candle.replace(tzinfo=None)
        before = 0
        filtered = []
        for n in news_items:
            pub = n.get("published")
            if pub and isinstance(pub, datetime):
                if pub.tzinfo is not None:
                    pub_naive = pub.replace(tzinfo=None)
                else:
                    pub_naive = pub
                if pub_naive < first_candle:
                    before += 1
                    continue
            filtered.append(n)
        if before:
            logger.info(f"Skipping {before} news items before first candle {first_candle}")
        news_items = filtered

    samples = label_by_price(news_items, candles, window, threshold, vol_mult, ticker=ticker)
    logger.info(f"Labeled: {sum(1 for s in samples if s.label=='positive')} pos / "
                f"{sum(1 for s in samples if s.label=='neutral')} neu / "
                f"{sum(1 for s in samples if s.label=='negative')} neg")

    if not samples:
        return

    rows = [s.to_row() for s in samples]
    df = pd.DataFrame(rows)
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = OUTPUT_DIR / f"{ticker}_{ts}.parquet"
    df.to_parquet(path, index=False)
    logger.info(f"Saved → {path} ({len(df)} samples)")
    return df


def merge_into_training():
    src = OUTPUT_DIR
    dst = HERE.parent / "finbert_sft" / "data"
    dst.mkdir(parents=True, exist_ok=True)

    parquets = sorted(src.glob("*.parquet"))
    if not parquets:
        logger.warning(f"No parquet files in {src}")
        return

    dfs = []
    for p in parquets:
        df = pd.read_parquet(p)
        ticker = p.stem.split("_")[0]
        df["ticker"] = ticker
        dfs.append(df)
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
    logger.info(f"Merged → train={len(train)} test={len(test)}")
    logger.info(f"Labels:\n{label_dist}")


def main():
    parser = argparse.ArgumentParser(description="Universal MOEX data collector")
    parser.add_argument("--ticker", type=str, default="SBER", help="MOEX ticker (SBER, T, YDEX, etc.)")
    parser.add_argument("--days", type=int, default=180, help="Days of history to collect")
    parser.add_argument("--window", type=int, default=60, help="Label window in minutes")
    parser.add_argument("--threshold", type=float, default=0.001, help="Price change threshold (default 0.1%%)")
    parser.add_argument("--vol-mult", type=float, default=1.2, help="Volume multiplier threshold")
    parser.add_argument("--keywords", nargs="*", help="RSS filter keywords (space-separated)")
    parser.add_argument("--topics", nargs="*", help="Perplexica search topics (space-separated)")
    parser.add_argument("--merge", action="store_true", help="Merge collected parquets into train/test")
    args = parser.parse_args()

    if args.merge:
        merge_into_training()
    else:
        run(
            ticker=args.ticker,
            days=args.days,
            window=args.window,
            threshold=args.threshold,
            vol_mult=args.vol_mult,
            keywords=args.keywords,
            topics=args.topics,
        )


if __name__ == "__main__":
    main()
