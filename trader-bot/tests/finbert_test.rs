use trader_bot::ml_inference::nlp::FinBertInference;

const MODEL_DIR: &str = "../models/finbert";

fn new_nlp() -> FinBertInference {
    FinBertInference::new(MODEL_DIR).expect("Failed to load FinBERT ONNX model")
}

#[test]
fn test_finbert_load_model() {
    let nlp = FinBertInference::new(MODEL_DIR);
    assert!(nlp.is_ok(), "Model should load without error");
}

#[test]
fn test_finbert_predict_returns_valid_result() {
    let nlp = new_nlp();
    let texts = [
        "компания показала рост выручки на 30 процентов",
        "компания объявила о дефолте по облигациям",
        "совет директоров проведет заседание в пятницу",
    ];
    for text in &texts {
        let result = nlp.predict(text).expect("Prediction failed");
        assert!(
            !result.label.is_empty(),
            "Label should not be empty for '{}'",
            text
        );
        assert!(
            result.confidence > 0.0,
            "Confidence should be positive for '{}'",
            text
        );
        assert_eq!(result.scores.len(), 3, "Should have 3 class scores");
    }
}

#[test]
fn test_finbert_long_text() {
    let nlp = new_nlp();
    let long_text = "компания показала рост прибыли. ".repeat(100);
    let result = nlp
        .predict(&long_text)
        .expect("Long text prediction failed");
    assert!(!result.label.is_empty());
    assert!(result.confidence > 0.0);
}

#[test]
fn test_finbert_empty_text() {
    let nlp = new_nlp();
    let result = nlp.predict("").expect("Empty text should not crash");
    assert!(!result.label.is_empty());
}

#[test]
fn test_finbert_sentiment_score_neutral() {
    let nlp = new_nlp();
    let result = nlp
        .predict("совет директоров проведет заседание в пятницу")
        .expect("Prediction failed");
    let score = result.sentiment_score();
    assert_eq!(score, 0.0, "Neutral should have score 0, got {}", score);
}

#[test]
fn test_finbert_special_characters() {
    let nlp = new_nlp();
    let texts = [
        "прибыль +30% !!!",
        "убыток -50% ?",
        "рост / падение / нейтрально",
    ];
    for text in &texts {
        let result = nlp.predict(text).expect("Prediction failed");
        assert!(!result.label.is_empty());
        assert!(result.confidence > 0.0);
    }
}

#[test]
fn test_finbert_repeated_calls() {
    let nlp = new_nlp();
    for _ in 0..10 {
        let result = nlp
            .predict("компания показала отличные результаты")
            .expect("Repeated prediction failed");
        assert!(!result.label.is_empty());
    }
}

#[test]
fn test_finbert_different_lengths() {
    let nlp = new_nlp();
    let texts = [
        "рост",
        "компания показала рост",
        "компания показала значительный рост прибыли в этом году",
        "компания показала значительный рост прибыли в этом году благодаря новым контрактам и расширению рынка",
    ];
    for text in &texts {
        let result = nlp.predict(text).expect("Prediction failed");
        assert!(!result.label.is_empty(), "Label empty for '{}'", text);
        assert!(result.confidence > 0.0, "Confidence zero for '{}'", text);
    }
}

#[test]
fn test_finbert_labels_are_valid() {
    let nlp = new_nlp();
    let valid = ["positive", "negative", "neutral"];
    for _ in 0..5 {
        let result = nlp.predict("тестовая новость").expect("Prediction failed");
        assert!(
            valid.contains(&result.label.as_str()),
            "Invalid label: {}",
            result.label
        );
    }
}

#[test]
fn test_finbert_scores_structure() {
    let nlp = new_nlp();
    let result = nlp.predict("тест").expect("Prediction failed");
    assert!(
        result.scores[0].is_finite(),
        "Negative score is not finite: {}",
        result.scores[0]
    );
    assert!(
        result.scores[1].is_finite(),
        "Neutral score is not finite: {}",
        result.scores[1]
    );
    assert!(
        result.scores[2].is_finite(),
        "Positive score is not finite: {}",
        result.scores[2]
    );
}

#[test]
fn test_finbert_concurrent_predictions() {
    use std::thread;
    let mut handles = vec![];
    for i in 0..4 {
        let text = format!("тестовая новость номер {}", i);
        handles.push(thread::spawn(move || {
            let nlp = new_nlp();
            nlp.predict(&text).expect("Thread prediction failed")
        }));
    }
    for handle in handles {
        let result = handle.join().expect("Thread panicked");
        assert!(!result.label.is_empty());
        assert!(result.confidence > 0.0);
    }
}
