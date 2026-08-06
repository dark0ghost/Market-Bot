/// Fundamental analysis result
#[derive(Debug, Clone)]
pub struct FundamentalAnalysis {
    pub ticker: String,
    pub company_name: String,
    pub market_cap: Option<f64>,
    pub valuation: ValuationMetrics,
    pub profitability: ProfitabilityMetrics,
    pub financial_health: FinancialHealthMetrics,
    pub growth: GrowthMetrics,
    pub dividends: Option<DividendMetrics>,
    pub overall_score: f64, // 0-100
    pub rating: CompanyRating,
    pub key_risks: Vec<String>,
    pub key_strengths: Vec<String>,
}

/// Valuation multiples
#[derive(Debug, Clone, Default)]
pub struct ValuationMetrics {
    pub pe_ratio: Option<f64>,
    pub forward_pe: Option<f64>,
    pub peg_ratio: Option<f64>,
    pub price_to_book: Option<f64>,
    pub price_to_sales: Option<f64>,
    pub ev_to_ebitda: Option<f64>,
}

/// Profitability metrics
#[derive(Debug, Clone, Default)]
pub struct ProfitabilityMetrics {
    pub gross_margin: Option<f64>,
    pub operating_margin: Option<f64>,
    pub net_margin: Option<f64>,
    pub roe: Option<f64>,
    pub roa: Option<f64>,
    pub roic: Option<f64>,
}

/// Financial health metrics
#[derive(Debug, Clone, Default)]
pub struct FinancialHealthMetrics {
    pub current_ratio: Option<f64>,
    pub quick_ratio: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub debt_to_assets: Option<f64>,
    pub interest_coverage: Option<f64>,
    pub free_cash_flow: Option<f64>,
}

/// Growth metrics
#[derive(Debug, Clone, Default)]
pub struct GrowthMetrics {
    pub revenue_growth_yoy: Option<f64>,
    pub earnings_growth_yoy: Option<f64>,
    pub revenue_growth_3y: Option<f64>,
    pub earnings_growth_3y: Option<f64>,
    pub revenue_growth_5y: Option<f64>,
    pub earnings_growth_5y: Option<f64>,
}

/// Dividend metrics
#[derive(Debug, Clone, Default)]
pub struct DividendMetrics {
    pub dividend_yield: Option<f64>,
    pub payout_ratio: Option<f64>,
    pub dividend_growth_3y: Option<f64>,
    pub consecutive_years: Option<u32>,
}

/// Company rating
#[derive(Debug, Clone, PartialEq)]
pub enum CompanyRating {
    Excellent, // 80-100
    Good,      // 60-79
    Fair,      // 40-59
    Poor,      // 20-39
    VeryPoor,  // 0-19
}

/// Fundamental analysis service
pub struct FundamentalAnalyzer {
    /// Industry average values for comparison
    industry_averages: IndustryAverages,
}

/// Industry averages
#[derive(Debug, Clone, Default)]
pub struct IndustryAverages {
    pub pe_ratio: Option<f64>,
    pub roe: Option<f64>,
    pub debt_to_equity: Option<f64>,
    pub revenue_growth: Option<f64>,
}

impl FundamentalAnalyzer {
    pub fn new(industry_averages: Option<IndustryAverages>) -> Self {
        FundamentalAnalyzer {
            industry_averages: industry_averages.unwrap_or_default(),
        }
    }

    /// Analyze fundamental metrics
    #[allow(clippy::too_many_arguments)]
    pub fn analyze(
        &self,
        ticker: &str,
        company_name: &str,
        valuation: ValuationMetrics,
        profitability: ProfitabilityMetrics,
        financial_health: FinancialHealthMetrics,
        growth: GrowthMetrics,
        dividends: Option<DividendMetrics>,
    ) -> FundamentalAnalysis {
        // Calculate scores for each category
        let valuation_score = self.score_valuation(&valuation);
        let profitability_score = self.score_profitability(&profitability);
        let financial_health_score = self.score_financial_health(&financial_health);
        let growth_score = self.score_growth(&growth);
        let dividend_score = dividends.as_ref().map_or(0.0, |d| self.score_dividends(d));

        // Overall score (weighted)
        let overall_score = (valuation_score * 0.25
            + profitability_score * 0.25
            + financial_health_score * 0.20
            + growth_score * 0.20
            + dividend_score * 0.10)
            .clamp(0.0, 100.0);

        let rating = self.score_to_rating(overall_score);
        let key_risks = self.identify_risks(&valuation, &profitability, &financial_health, &growth);
        let key_strengths =
            self.identify_strengths(&valuation, &profitability, &financial_health, &growth);

        // Market cap calculation (if data available)
        let market_cap = None; // Requires external data

        FundamentalAnalysis {
            ticker: ticker.to_string(),
            company_name: company_name.to_string(),
            market_cap,
            valuation,
            profitability,
            financial_health,
            growth,
            dividends,
            overall_score,
            rating,
            key_risks,
            key_strengths,
        }
    }

    /// Valuation scoring
    const fn score_valuation(&self, metrics: &ValuationMetrics) -> f64 {
        let mut score = 50.0;
        let mut count = 0;

        // P/E analysis
        if let Some(pe) = metrics.pe_ratio {
            if let Some(industry_pe) = self.industry_averages.pe_ratio {
                if pe < industry_pe * 0.8 {
                    score += 20.0; // Undervalued
                } else if pe > industry_pe * 1.2 {
                    score -= 20.0; // Overvalued
                }
            } else {
                if pe < 10.0 {
                    score += 15.0;
                } else if pe > 25.0 {
                    score -= 15.0;
                }
            }
            count += 1;
        }

        // PEG analysis
        if let Some(peg) = metrics.peg_ratio {
            if peg < 1.0 {
                score += 15.0;
            } else if peg > 2.0 {
                score -= 15.0;
            }
            count += 1;
        }

        // P/B analysis
        if let Some(pb) = metrics.price_to_book {
            if pb < 1.5 {
                score += 10.0;
            } else if pb > 3.0 {
                score -= 10.0;
            }
            count += 1;
        }

        if count > 0 {
            score / (count as f64 / 3.0)
        } else {
            50.0
        }
    }

    /// Profitability scoring
    const fn score_profitability(&self, metrics: &ProfitabilityMetrics) -> f64 {
        let mut score = 50.0;
        let mut count = 0;

        // ROE analysis
        if let Some(roe) = metrics.roe {
            if let Some(industry_roe) = self.industry_averages.roe {
                if roe > industry_roe * 1.2 {
                    score += 20.0;
                } else if roe < industry_roe * 0.8 {
                    score -= 20.0;
                }
            } else {
                if roe > 15.0 {
                    score += 20.0;
                } else if roe < 5.0 {
                    score -= 20.0;
                }
            }
            count += 1;
        }

        // Net margin analysis
        if let Some(margin) = metrics.net_margin {
            if margin > 20.0 {
                score += 15.0;
            } else if margin < 5.0 {
                score -= 15.0;
            }
            count += 1;
        }

        // ROIC analysis
        if let Some(roic) = metrics.roic {
            if roic > 15.0 {
                score += 15.0;
            } else if roic < 5.0 {
                score -= 15.0;
            }
            count += 1;
        }

        if count > 0 {
            score / (count as f64 / 3.0)
        } else {
            50.0
        }
    }

    /// Financial health scoring
    const fn score_financial_health(&self, metrics: &FinancialHealthMetrics) -> f64 {
        let mut score = 50.0;
        let mut count = 0;

        // Debt/Equity analysis
        if let Some(dte) = metrics.debt_to_equity {
            if let Some(industry_dte) = self.industry_averages.debt_to_equity {
                if dte < industry_dte * 0.8 {
                    score += 20.0;
                } else if dte > industry_dte * 1.2 {
                    score -= 20.0;
                }
            } else {
                if dte < 0.5 {
                    score += 20.0;
                } else if dte > 2.0 {
                    score -= 20.0;
                }
            }
            count += 1;
        }

        // Current ratio analysis
        if let Some(cr) = metrics.current_ratio {
            if cr > 1.5 {
                score += 15.0;
            } else if cr < 1.0 {
                score -= 20.0;
            }
            count += 1;
        }

        // Interest coverage analysis
        if let Some(ic) = metrics.interest_coverage {
            if ic > 5.0 {
                score += 15.0;
            } else if ic < 2.0 {
                score -= 20.0;
            }
            count += 1;
        }

        if count > 0 {
            score / (count as f64 / 3.0)
        } else {
            50.0
        }
    }

    /// Growth scoring
    const fn score_growth(&self, metrics: &GrowthMetrics) -> f64 {
        let mut score = 50.0;
        let mut count = 0;

        // Revenue growth YoY
        if let Some(growth) = metrics.revenue_growth_yoy {
            if let Some(industry_growth) = self.industry_averages.revenue_growth {
                if growth > industry_growth * 1.2 {
                    score += 20.0;
                } else if growth < industry_growth * 0.8 {
                    score -= 20.0;
                }
            } else {
                if growth > 15.0 {
                    score += 20.0;
                } else if growth < 0.0 {
                    score -= 20.0;
                }
            }
            count += 1;
        }

        // Earnings growth YoY
        if let Some(growth) = metrics.earnings_growth_yoy {
            if growth > 15.0 {
                score += 20.0;
            } else if growth < 0.0 {
                score -= 20.0;
            }
            count += 1;
        }

        // 3-year growth
        if let Some(growth) = metrics.revenue_growth_3y {
            if growth > 10.0 {
                score += 15.0;
            } else if growth < 0.0 {
                score -= 15.0;
            }
            count += 1;
        }

        if count > 0 {
            score / (count as f64 / 3.0)
        } else {
            50.0
        }
    }

    /// Dividend scoring
    const fn score_dividends(&self, metrics: &DividendMetrics) -> f64 {
        let mut score: f64 = 50.0;

        // Dividend yield
        if let Some(yield_pct) = metrics.dividend_yield {
            if yield_pct > 5.0 {
                score += 20.0;
            } else if yield_pct > 3.0 {
                score += 10.0;
            } else if yield_pct < 1.0 {
                score -= 10.0;
            }
        }

        // Payout ratio
        if let Some(payout) = metrics.payout_ratio {
            if payout < 50.0 {
                score += 15.0; // Sustainable payout
            } else if payout > 80.0 {
                score -= 15.0; // Too high
            }
        }

        // Dividend growth
        if let Some(growth) = metrics.dividend_growth_3y {
            if growth > 5.0 {
                score += 15.0;
            } else if growth < 0.0 {
                score -= 15.0;
            }
        }

        score.min(100.0).max(0.0)
    }

    /// Convert score to rating
    const fn score_to_rating(&self, score: f64) -> CompanyRating {
        match score {
            s if s >= 80.0 => CompanyRating::Excellent,
            s if s >= 60.0 => CompanyRating::Good,
            s if s >= 40.0 => CompanyRating::Fair,
            s if s >= 20.0 => CompanyRating::Poor,
            _ => CompanyRating::VeryPoor,
        }
    }

    /// Identify risks
    fn identify_risks(
        &self,
        valuation: &ValuationMetrics,
        profitability: &ProfitabilityMetrics,
        financial_health: &FinancialHealthMetrics,
        growth: &GrowthMetrics,
    ) -> Vec<String> {
        let mut risks = Vec::new();

        // Valuation risks
        if let Some(pe) = valuation.pe_ratio
            && pe > 30.0
        {
            risks.push("High P/E - overvaluation risk".to_string());
        }

        // Profitability risks
        if let Some(roe) = profitability.roe
            && roe < 5.0
        {
            risks.push("Low return on equity (ROE)".to_string());
        }

        // Financial health risks
        if let Some(dte) = financial_health.debt_to_equity
            && dte > 2.0
        {
            risks.push("High debt burden".to_string());
        }

        if let Some(cr) = financial_health.current_ratio
            && cr < 1.0
        {
            risks.push("Liquidity concerns".to_string());
        }

        // Growth risks
        if let Some(growth) = growth.revenue_growth_yoy
            && growth < 0.0
        {
            risks.push("Revenue decline".to_string());
        }

        risks
    }

    /// Identify strengths
    fn identify_strengths(
        &self,
        valuation: &ValuationMetrics,
        profitability: &ProfitabilityMetrics,
        financial_health: &FinancialHealthMetrics,
        growth: &GrowthMetrics,
    ) -> Vec<String> {
        let mut strengths = Vec::new();

        // Valuation advantages
        if let Some(pe) = valuation.pe_ratio
            && pe < 10.0
        {
            strengths.push("Low P/E - potential undervaluation".to_string());
        }

        // Profitability advantages
        if let Some(roe) = profitability.roe
            && roe > 15.0
        {
            strengths.push("High return on equity (ROE)".to_string());
        }

        if let Some(margin) = profitability.net_margin
            && margin > 20.0
        {
            strengths.push("High net margin".to_string());
        }

        // Financial health advantages
        if let Some(dte) = financial_health.debt_to_equity
            && dte < 0.5
        {
            strengths.push("Low debt burden".to_string());
        }

        // Growth advantages
        if let Some(growth) = growth.revenue_growth_yoy
            && growth > 15.0
        {
            strengths.push("High revenue growth".to_string());
        }

        strengths
    }
}

impl Default for FundamentalAnalyzer {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_conversion() {
        let analyzer = FundamentalAnalyzer::default();

        assert_eq!(analyzer.score_to_rating(85.0), CompanyRating::Excellent);
        assert_eq!(analyzer.score_to_rating(70.0), CompanyRating::Good);
        assert_eq!(analyzer.score_to_rating(50.0), CompanyRating::Fair);
        assert_eq!(analyzer.score_to_rating(30.0), CompanyRating::Poor);
        assert_eq!(analyzer.score_to_rating(10.0), CompanyRating::VeryPoor);
    }

    #[test]
    fn test_fundamental_analysis() {
        let analyzer = FundamentalAnalyzer::default();

        let valuation = ValuationMetrics {
            pe_ratio: Some(8.0),
            ..Default::default()
        };

        let profitability = ProfitabilityMetrics {
            roe: Some(20.0),
            net_margin: Some(25.0),
            ..Default::default()
        };

        let financial_health = FinancialHealthMetrics {
            debt_to_equity: Some(0.3),
            current_ratio: Some(2.0),
            ..Default::default()
        };

        let growth = GrowthMetrics {
            revenue_growth_yoy: Some(18.0),
            earnings_growth_yoy: Some(20.0),
            ..Default::default()
        };

        let result = analyzer.analyze(
            "TEST",
            "Test Company",
            valuation,
            profitability,
            financial_health,
            growth,
            None,
        );

        assert!(result.overall_score > 60.0);
        assert!(!result.key_strengths.is_empty());
    }
}
