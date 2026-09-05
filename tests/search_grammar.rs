use serde_json::Value;
use std::fs;
use std::path::Path;

fn fixture() -> Vec<Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("tests/fixtures/search_queries.js")).unwrap();
    let start = source.find("var QUERIES = ").expect("fixture header") + "var QUERIES = ".len();
    let json = source[start..].trim().trim_end_matches(';');
    serde_json::from_str(json).expect("fixture is JSON after the header")
}

#[test]
fn the_rust_grammar_agrees_with_the_shared_fixture() {
    let rows = fixture();
    assert!(rows.len() >= 10);
    for row in rows {
        let query = row["query"].as_str().unwrap();
        let parsed = fileblade::search::query_document(query);
        assert_eq!(parsed["ok"], true, "{query}: {parsed}");
        let terms = parsed["terms"].as_array().unwrap();
        let expected_terms = row["terms"].as_array().unwrap();
        assert_eq!(
            terms.len(),
            expected_terms.len(),
            "term count for {query}: {parsed}"
        );
        for (term, expected) in terms.iter().zip(expected_terms) {
            for key in ["text", "negate", "exact", "field"] {
                assert_eq!(term[key], expected[key], "{query} term {key}: {parsed}");
            }
        }
        let filters = parsed["filters"].as_array().unwrap();
        let expected_filters = row["filters"].as_array().unwrap();
        assert_eq!(
            filters.len(),
            expected_filters.len(),
            "filter count for {query}: {parsed}"
        );
        for (filter, expected) in filters.iter().zip(expected_filters) {
            for key in ["key", "value", "negate"] {
                assert_eq!(filter[key], expected[key], "{query} filter {key}: {parsed}");
            }
        }
    }
}
