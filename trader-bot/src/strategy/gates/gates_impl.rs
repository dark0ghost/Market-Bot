use crate::analysis::NewsSentiment;
use crate::analysis::detectors::{
    Candle, detect_cisd, detect_displacement, detect_fvg, detect_swings, update_fvg_states,
};
use crate::strategy::gates::{Gate, GateContext, GateResult};

// ─── Regime Gate ──────────────────────────────────────────────────────

pub struct RegimeGate {
    name: String,
    allowed_regimes: Vec<String>,
}

impl RegimeGate {
    pub fn new(allowed: &[&str]) -> Self {
        RegimeGate {
            name: format!("regime[{}]", allowed.join(",")),
            allowed_regimes: allowed.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Gate for RegimeGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        match &ctx.market_regime {
            Some(regime) if self.allowed_regimes.iter().any(|a| a == regime) => GateResult::Pass,
            Some(regime) => GateResult::Fail(format!("regime {} not allowed", regime)),
            None => GateResult::Fail("unknown regime".into()),
        }
    }
}

// ─── Trend Gate ───────────────────────────────────────────────────────

pub struct TrendGate {
    name: String,
    allowed_trends: Vec<String>,
}

impl TrendGate {
    pub fn new(allowed: &[&str]) -> Self {
        TrendGate {
            name: format!("trend[{}]", allowed.join(",")),
            allowed_trends: allowed.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Gate for TrendGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        match &ctx.trend {
            Some(trend) if self.allowed_trends.iter().any(|a| a == trend) => GateResult::Pass,
            Some(trend) => GateResult::Fail(format!("trend {} not allowed", trend)),
            None => GateResult::Fail("unknown trend".into()),
        }
    }
}

// ─── Swing Gate ──────────────────────────────────────────────────────

pub struct SwingCheckGate {
    name: String,
    /// Minimum number of swing highs/lows to consider the market "active"
    min_swings: usize,
}

impl SwingCheckGate {
    pub fn new(min_swings: usize) -> Self {
        SwingCheckGate {
            name: format!("swings>={}", min_swings),
            min_swings,
        }
    }
}

impl Gate for SwingCheckGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        if ctx.swings.len() >= self.min_swings {
            GateResult::Pass
        } else {
            GateResult::Fail(format!(
                "only {} swings (need >= {})",
                ctx.swings.len(),
                self.min_swings
            ))
        }
    }
}

// ─── FVG Gate ─────────────────────────────────────────────────────────

pub struct FvgGate {
    name: String,
    /// Require at least this many unconsumed FVGs
    min_fvgs: usize,
}

impl FvgGate {
    pub fn new(min_fvgs: usize) -> Self {
        FvgGate {
            name: format!("fvg>={}", min_fvgs),
            min_fvgs,
        }
    }
}

impl Gate for FvgGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        let active: Vec<_> = ctx
            .fvgs
            .iter()
            .filter(|f| !f.state.filled && !f.state.inversed)
            .collect();
        if active.len() >= self.min_fvgs {
            GateResult::Pass
        } else {
            GateResult::Fail(format!(
                "only {} active FVGs (need >= {})",
                active.len(),
                self.min_fvgs
            ))
        }
    }
}

// ─── CISD Gate ────────────────────────────────────────────────────────

pub struct CisdGate;

impl Gate for CisdGate {
    fn name(&self) -> &str {
        "cisd"
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        if ctx.cisd_signals.is_empty() {
            GateResult::Fail("no CISD signal".into())
        } else {
            GateResult::Pass
        }
    }
}

// ─── Displacement Gate ────────────────────────────────────────────────

pub struct DisplacementGate {
    name: String,
    min_displacements: usize,
}

impl DisplacementGate {
    pub fn new(min: usize) -> Self {
        DisplacementGate {
            name: format!("displacement>={}", min),
            min_displacements: min,
        }
    }
}

impl Gate for DisplacementGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        if ctx.displacements.len() >= self.min_displacements {
            GateResult::Pass
        } else {
            GateResult::Fail(format!(
                "only {} displacements (need >= {})",
                ctx.displacements.len(),
                self.min_displacements
            ))
        }
    }
}

// ─── Sentiment Gate ───────────────────────────────────────────────────

pub struct SentimentGate {
    name: String,
    min_score: f64,
}

impl SentimentGate {
    pub fn new(min_score: f64) -> Self {
        SentimentGate {
            name: format!("sentiment>={:.1}", min_score),
            min_score,
        }
    }
}

impl Gate for SentimentGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        match &ctx.sentiment {
            Some(s) if s.sentiment_score >= self.min_score => GateResult::Pass,
            Some(s) => GateResult::Fail(format!(
                "sentiment {:.2} < {:.2}",
                s.sentiment_score, self.min_score
            )),
            None => GateResult::Pass, // no news = no veto
        }
    }
}

/// Build a populated GateContext by running detectors on candle data.
/// This is a convenience to wire detectors into the gate pipeline.
pub fn build_gate_context(
    ticker: &str,
    candles: &[Candle],
    current_price: f64,
    sentiment: Option<NewsSentiment>,
    market_regime: Option<String>,
    trend: Option<String>,
) -> GateContext {
    let swings = detect_swings(candles, 3);
    let mut fvgs = detect_fvg(candles);
    update_fvg_states(&mut fvgs, candles);
    let cisd_signals = detect_cisd(candles, &swings);
    let displacements = detect_displacement(candles, 5, 2.0);

    let mut ctx = GateContext::new(ticker);
    ctx.candles = candles.to_vec();
    ctx.swings = swings;
    ctx.fvgs = fvgs;
    ctx.cisd_signals = cisd_signals;
    ctx.displacements = displacements;
    ctx.current_price = current_price;
    ctx.sentiment = sentiment;
    ctx.market_regime = market_regime;
    ctx.trend = trend;
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::detectors::Candle;

    fn dummy_candles(n: usize) -> Vec<Candle> {
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(Candle {
                open: 100.0,
                high: 101.0 + (i % 5) as f64,
                low: 99.0 - (i % 3) as f64,
                close: 100.0 + (i % 2) as f64,
                volume: 0.0,
            });
        }
        v
    }

    #[test]
    fn test_regime_gate_pass() {
        let gate = RegimeGate::new(&["trending"]);
        let mut ctx = GateContext::new("AAPL");
        ctx.market_regime = Some("trending".into());
        assert!(gate.evaluate(&ctx).is_pass());
    }

    #[test]
    fn test_regime_gate_fail() {
        let gate = RegimeGate::new(&["trending"]);
        let mut ctx = GateContext::new("AAPL");
        ctx.market_regime = Some("volatile".into());
        assert!(!gate.evaluate(&ctx).is_pass());
    }

    #[test]
    fn test_sentiment_gate_pass() {
        let gate = SentimentGate::new(0.3);
        let mut ctx = GateContext::new("AAPL");
        ctx.sentiment = Some(NewsSentiment {
            ticker: "AAPL".into(),
            overall_sentiment: crate::analysis::Sentiment::Positive,
            sentiment_score: 0.5,
            articles_count: 1,
            articles: vec![],
            key_events: vec![],
        });
        assert!(gate.evaluate(&ctx).is_pass());
    }

    #[test]
    fn test_build_context_runs_detectors() {
        let candles = dummy_candles(30);
        let ctx = build_gate_context("AAPL", &candles, 100.0, None, None, None);
        assert!(
            !ctx.swings.is_empty()
                || !ctx.fvgs.is_empty()
                || ctx.cisd_signals.is_empty()
                || ctx.displacements.is_empty()
        );
        assert_eq!(ctx.ticker, "AAPL");
    }
}
