use crate::analysis::NewsSentiment;
use crate::analysis::detectors::{CISDSignal, Candle, Displacement, FairValueGap, Swing};

pub mod gates_impl;

/// A single pass/fail gate in the screening pipeline.
pub trait Gate: Send + Sync {
    fn name(&self) -> &str;
    fn evaluate(&self, ctx: &GateContext) -> GateResult;
}

#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Pass,
    Fail(String),
}

impl GateResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, GateResult::Pass)
    }
}

#[derive(Debug, Clone)]
pub struct GateContext {
    pub ticker: String,
    pub candles: Vec<Candle>,
    pub swings: Vec<Swing>,
    pub fvgs: Vec<FairValueGap>,
    pub cisd_signals: Vec<CISDSignal>,
    pub displacements: Vec<Displacement>,
    pub current_price: f64,
    pub sentiment: Option<NewsSentiment>,
    pub market_regime: Option<String>,
    pub trend: Option<String>,
}

impl GateContext {
    pub fn new(ticker: &str) -> Self {
        GateContext {
            ticker: ticker.to_string(),
            candles: vec![],
            swings: vec![],
            fvgs: vec![],
            cisd_signals: vec![],
            displacements: vec![],
            current_price: 0.0,
            sentiment: None,
            market_regime: None,
            trend: None,
        }
    }
}

/// Run a sequence of gates. Short-circuits on first Fail.
pub fn run_gates(gates: &[Box<dyn Gate>], ctx: &GateContext) -> Vec<GateResult> {
    let mut results = Vec::new();
    for gate in gates {
        let result = gate.evaluate(ctx);
        let is_pass = result.is_pass();
        results.push(result);
        if !is_pass {
            break;
        }
    }
    results
}

/// Wrap an LLM query as a veto-only gate.
/// The LLM is called last and can only reject (Fail) — it cannot force a Pass.
pub struct LlmVetoGate {
    name: String,
    llm: Box<dyn crate::agent::LlmQuery>,
}

impl LlmVetoGate {
    pub fn new(name: &str, llm: Box<dyn crate::agent::LlmQuery>) -> Self {
        LlmVetoGate {
            name: name.to_string(),
            llm,
        }
    }
}

impl Gate for LlmVetoGate {
    fn name(&self) -> &str {
        &self.name
    }

    fn evaluate(&self, ctx: &GateContext) -> GateResult {
        let prompt = format!(
            "You are a conservative risk manager. Review this setup and respond ONLY with YES or NO.

Ticker: {}
Price: {:.2}
Trend: {}
Regime: {:?}
Swings detected: {}
FVG patterns: {}
CISD signals: {}
Displacement candles: {}

Would you veto this trade? Answer YES to reject, NO to allow.",
            ctx.ticker,
            ctx.current_price,
            ctx.trend.as_deref().unwrap_or("unknown"),
            ctx.market_regime,
            ctx.swings.len(),
            ctx.fvgs.len(),
            ctx.cisd_signals.len(),
            ctx.displacements.len(),
        );

        match tokio::runtime::Handle::current().block_on(self.llm.query(prompt)) {
            Ok(response) => {
                let upper = response.trim().to_uppercase();
                if upper.starts_with("Y") {
                    GateResult::Fail(format!("LLM veto: {}", response.trim()))
                } else {
                    GateResult::Pass
                }
            }
            Err(e) => {
                log::warn!("LLM veto error (fail-open): {}", e);
                GateResult::Pass
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PassGate;
    impl Gate for PassGate {
        fn name(&self) -> &str {
            "pass"
        }
        fn evaluate(&self, _: &GateContext) -> GateResult {
            GateResult::Pass
        }
    }

    struct FailGate;
    impl Gate for FailGate {
        fn name(&self) -> &str {
            "fail"
        }
        fn evaluate(&self, _: &GateContext) -> GateResult {
            GateResult::Fail("nope".into())
        }
    }

    #[test]
    fn test_all_pass() {
        let gates: Vec<Box<dyn Gate>> = vec![Box::new(PassGate), Box::new(PassGate)];
        let ctx = GateContext::new("AAPL");
        let results = run_gates(&gates, &ctx);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_pass()));
    }

    #[test]
    fn test_short_circuit_on_fail() {
        let gates: Vec<Box<dyn Gate>> =
            vec![Box::new(PassGate), Box::new(FailGate), Box::new(PassGate)];
        let ctx = GateContext::new("AAPL");
        let results = run_gates(&gates, &ctx);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_pass());
        assert!(!results[1].is_pass());
    }

    #[test]
    fn test_gate_result_is_pass() {
        assert!(GateResult::Pass.is_pass());
        assert!(!GateResult::Fail("x".into()).is_pass());
    }
}
