---
hide:
  - navigation
---

# AI Trade Bot Documentation

Welcome to the AI Trade Bot documentation — a comprehensive AI-powered trading system for the Moscow Exchange.

## Quick Start

### 5-Minute Setup

```bash
# 1. Clone the repository
git clone https://github.com/dark0ghost/ai-trader-bot.git
cd ai-trade-bot

# 2. Set up your token
export API_TOKEN="your_tinkoff_token"

# 3. Start Ollama (for LLM analysis)
docker-compose up -d

# 4. Run the bot
cargo run -p trader-bot
```

## Documentation Sections

### Getting Started

- **[What is AI Trade Bot?](getting-started/what-is.md)** — Feature overview
- **[Quick Start](getting-started/quickstart.md)** — Installation and launch

### User Guide

- **[Configuration](user-guide/configuration.md)** — Bot configuration

### Developer Guide

- **[API Documentation](developer-guide/api.md)** — API reference

### Strategies

- **[Grid Bot](strategies/grid-bot.md)** — Grid trading

## Features

| Feature | Description | Status |
|---------|-------------|--------|
| AI Analysis | LLM news analysis and decision making | ✅ |
| Technical Analysis | RSI, MACD, Bollinger Bands | ✅ |
| Fundamental Analysis | P/E, ROE, D/E, growth | ✅ |
| Grid Strategy | Automated order grid | ✅ |
| Risk Management | Stop Loss, Take Profit, limits | ✅ |
| Tinkoff API | Moscow Exchange integration | ✅ |

## Support

- [GitHub Issues](https://github.com/dark0ghost/ai-trader-bot/issues) — Report a bug
- [GitHub Issues](https://github.com/dark0ghost/ai-trader-bot/issues) — Report a bug

## License

MIT License — see [LICENSE](../../LICENSE)

---

**Last updated:** February 22, 2026  
**Documentation version:** 0.2.0
