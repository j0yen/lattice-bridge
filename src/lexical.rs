//! Lexical scoring: normalized edit distance + token overlap between label sets.

use strsim::normalized_levenshtein;

/// Normalize a label for comparison: lowercase, strip punctuation, collapse whitespace.
pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Token set of a normalized label.
fn tokens(s: &str) -> std::collections::HashSet<String> {
    s.split_whitespace().map(str::to_string).collect()
}

/// Token-overlap Jaccard between two normalized strings.
pub fn token_overlap(a: &str, b: &str) -> f64 {
    let ta = tokens(a);
    let tb = tokens(b);
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let intersection = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    intersection / union
}

/// Best pairwise lexical score between two sets of labels.
///
/// Tries every (a_label, b_label) combination and returns the max of:
///   0.7 * normalized_edit + 0.3 * token_overlap
pub fn best_score(a_labels: &[String], b_labels: &[String]) -> f64 {
    let mut best = 0.0_f64;
    for al in a_labels {
        let na = normalize(al);
        for bl in b_labels {
            let nb = normalize(bl);
            let edit = normalized_levenshtein(&na, &nb);
            let tok = token_overlap(&na, &nb);
            let score = 0.7 * edit + 0.3 * tok;
            if score > best {
                best = score;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lowercases_and_strips() {
        assert_eq!(normalize("Heart-Disease"), "heart disease");
        assert_eq!(normalize("  Foo  Bar "), "foo bar");
    }

    #[test]
    fn token_overlap_identical() {
        assert!((token_overlap("foo bar", "foo bar") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn token_overlap_disjoint() {
        assert!((token_overlap("foo", "bar") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn best_score_exact_match() {
        let a = vec!["Person".to_string()];
        let b = vec!["Person".to_string()];
        assert!((best_score(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn best_score_no_overlap() {
        let a = vec!["Quasar".to_string()];
        let b = vec!["Disposition".to_string()];
        let s = best_score(&a, &b);
        assert!(s < 0.5, "expected low score, got {s}");
    }
}
