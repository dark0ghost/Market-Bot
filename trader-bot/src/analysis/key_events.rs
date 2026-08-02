//! Shared key-event extraction (was duplicated in `news.rs` and `finbert.rs`).
//!
//! Keyword-based detection of market-moving corporate events from free text.

/// Extract market-moving key events from arbitrary text.
///
/// Returns a sorted, deduplicated list of human-readable event labels.
pub fn extract_key_events(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut events = Vec::new();

    if text_lower.contains("reconversion") || text_lower.contains("conversion") {
        events.push("Stock reconversion".to_string());
    }
    if text_lower.contains("dividend") {
        events.push("Dividend payments".to_string());
    }
    if text_lower.contains("report") || text_lower.contains("financial result") {
        events.push("Financial reporting".to_string());
    }
    if text_lower.contains("sanction") || text_lower.contains("restrict") {
        events.push("Sanctions pressure".to_string());
    }
    if text_lower.contains("merger") || text_lower.contains("acquisition") {
        events.push("M&A activity".to_string());
    }
    if text_lower.contains("buyback") || text_lower.contains("repurchase") {
        events.push("Share buyback".to_string());
    }
    if text_lower.contains("guidance") || text_lower.contains("forecast") {
        events.push("Guidance update".to_string());
    }

    events.sort();
    events.dedup();
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dividend_and_report() {
        let ev = extract_key_events("Company reported dividend increase and financial results");
        assert!(ev.contains(&"Dividend payments".to_string()));
        assert!(ev.contains(&"Financial reporting".to_string()));
    }

    #[test]
    fn detects_sanctions() {
        let ev = extract_key_events("New sanctions restrict exports");
        assert!(ev.contains(&"Sanctions pressure".to_string()));
    }

    #[test]
    fn dedups_repeated_events() {
        let ev = extract_key_events("dividend dividend dividend");
        assert_eq!(ev, vec!["Dividend payments".to_string()]);
    }

    #[test]
    fn no_events_for_neutral_text() {
        let ev = extract_key_events("The weather is nice today");
        assert!(ev.is_empty());
    }
}
