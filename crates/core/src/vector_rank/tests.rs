use super::embedding::tokenize_terms;
use super::{ann_bucket_keys, embed_for_index, semantic_model_name, vector_dim, vector_to_json};

#[test]
fn tokenize_terms_is_unicode_aware() {
    let terms = tokenize_terms(
        "Stra\u{00DF}e \u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}_\u{043C}\u{0438}\u{0440} \u{6771}\u{4EAC}",
    );
    assert!(terms.contains(&"stra\u{00DF}e".to_string()));
    assert!(terms.contains(
        &"\u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}_\u{043C}\u{0438}\u{0440}".to_string()
    ));
    assert!(terms.contains(&"\u{6771}\u{4EAC}".to_string()));
}

#[test]
fn semantic_model_name_is_non_empty() {
    assert!(!semantic_model_name().trim().is_empty());
}

#[test]
fn embed_for_index_returns_expected_dimension_and_finite_values() {
    let vector = embed_for_index("GraphRef parse_vector line column");
    assert_eq!(vector.len(), vector_dim());
    assert!(vector.iter().all(|value| value.is_finite()));
}

#[test]
fn vector_to_json_round_trips_live_embeddings() {
    let vector = embed_for_index("sample_symbol GraphRef");
    let raw = vector_to_json(&vector).expect("serialize vector");
    let decoded = serde_json::from_str::<Vec<f32>>(&raw).expect("decode vector json");
    assert_eq!(decoded, vector);
    assert_eq!(decoded.len(), vector_dim());
}

#[test]
fn ann_bucket_keys_returns_empty_for_empty_vector() {
    assert!(ann_bucket_keys(&[]).is_empty());
}

#[test]
fn ann_bucket_keys_are_deterministic() {
    let mut vector = vec![0.0_f32; vector_dim()];
    vector[3] = 0.1;
    vector[19] = -0.2;
    vector[54] = 0.7;
    let first = ann_bucket_keys(&vector);
    let second = ann_bucket_keys(&vector);
    assert_eq!(first, second);
    assert_eq!(first.len(), 4);
}
