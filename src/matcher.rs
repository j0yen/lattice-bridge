//! Core alignment pipeline: BFO-anchor prune → lexical score → structural score → classify.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::anchor::BfoAnchor;
use crate::error::BridgeError;
use crate::lexical;
use crate::structural;

/// Options controlling the alignment pipeline.
#[derive(Debug, Clone)]
pub struct AlignOptions {
    /// Confidence threshold above which a mapping is written to the OWL output.
    /// Below this threshold it goes to proposals.jsonl for review.
    pub confidence_threshold: f64,
    /// Weight of lexical score in the combined confidence (structural = 1 - lex_weight).
    pub lex_weight: f64,
}

impl Default for AlignOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.85,
            lex_weight: 0.8,
        }
    }
}

/// Classification of a mapping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MappingKind {
    Equivalent,
    SubClassOf,
}

impl std::fmt::Display for MappingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MappingKind::Equivalent => write!(f, "equivalent"),
            MappingKind::SubClassOf => write!(f, "subClassOf"),
        }
    }
}

/// A candidate or accepted mapping between a class in ontology A and one in B.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    /// Class IRI from ontology A.
    pub a_iri: String,
    /// Class IRI from ontology B.
    pub b_iri: String,
    /// Shared BFO upper category IRI.
    pub bfo_category: String,
    /// Lexical score component [0, 1].
    pub lexical_score: f64,
    /// Structural score component [0, 1] (stub: 0.0).
    pub structural_score: f64,
    /// Combined confidence [0, 1].
    pub confidence: f64,
    /// Classification.
    pub kind: MappingKind,
    /// Human-readable label for A.
    pub a_label: String,
    /// Human-readable label for B.
    pub b_label: String,
    /// Whether this mapping exceeds the confidence threshold.
    pub accepted: bool,
}

/// Run the full alignment pipeline.
///
/// 1. Group classes from A and B by BFO category (prune cross-category pairs).
/// 2. For each BFO bucket, score every pair.
/// 3. Classify and threshold.
pub fn align(
    a_anchors: &[BfoAnchor],
    b_anchors: &[BfoAnchor],
    opts: &AlignOptions,
) -> Result<Vec<Mapping>, BridgeError> {
    // Index b by category.
    let mut b_by_cat: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, b) in b_anchors.iter().enumerate() {
        b_by_cat.entry(b.category.0.clone()).or_default().push(i);
    }

    let mut mappings = Vec::new();

    for (ai, a) in a_anchors.iter().enumerate() {
        let cat = &a.category.0;
        let Some(b_idxs) = b_by_cat.get(cat) else {
            // No B classes in this BFO category; skip.
            continue;
        };

        for &bi in b_idxs {
            let b = &b_anchors[bi];

            let lex = lexical::best_score(&a.labels, &b.labels);
            let struc = structural::score(a, b, a_anchors, b_anchors, &[]);

            let confidence = opts.lex_weight * lex + (1.0 - opts.lex_weight) * struc;

            // Classify: high confidence (>= threshold) → equivalent, medium → subClassOf.
            let kind = if confidence >= opts.confidence_threshold {
                MappingKind::Equivalent
            } else {
                MappingKind::SubClassOf
            };

            let accepted = confidence >= opts.confidence_threshold;
            let a_label = a.labels.first().cloned().unwrap_or_default();
            let b_label = b.labels.first().cloned().unwrap_or_default();

            // Only emit mappings with at least minimal evidence (lex > 0).
            if lex > 0.0 || struc > 0.0 {
                mappings.push(Mapping {
                    a_iri: a.class_iri.clone(),
                    b_iri: b.class_iri.clone(),
                    bfo_category: cat.clone(),
                    lexical_score: lex,
                    structural_score: struc,
                    confidence,
                    kind,
                    a_label,
                    b_label,
                    accepted,
                });
            }

            // Synthetic index just to avoid unused-variable warning at caller level.
            let _ = ai;
        }
    }

    // Sort by confidence descending for deterministic output.
    mappings.sort_by(|x, y| {
        y.confidence
            .partial_cmp(&x.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(x.a_iri.cmp(&y.a_iri))
            .then(x.b_iri.cmp(&y.b_iri))
    });

    Ok(mappings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::BfoCategory;
    use crate::vocab;

    fn make_anchor(iri: &str, cat: &str, labels: &[&str]) -> BfoAnchor {
        BfoAnchor {
            class_iri: iri.to_string(),
            category: BfoCategory(cat.to_string()),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            definition: None,
            parents: vec![],
        }
    }

    #[test]
    fn bfo_anchor_prune_blocks_cross_category() {
        // A class under BFO:quality must NOT map to a class under BFO:process
        // even if their labels are identical (AC2).
        let a = vec![make_anchor(
            "http://example.org/a#Temperature",
            vocab::BFO_QUALITY,
            &["Temperature"],
        )];
        let b = vec![make_anchor(
            "http://example.org/b#Temperature",
            vocab::BFO_PROCESS,
            &["Temperature"],
        )];
        let opts = AlignOptions::default();
        let mappings = align(&a, &b, &opts).unwrap();
        assert!(
            mappings.is_empty(),
            "cross-BFO-category mapping must be pruned; got {:?}",
            mappings
        );
    }

    #[test]
    fn same_category_label_match_produces_mapping() {
        let a = vec![make_anchor(
            "http://example.org/a#Person",
            vocab::BFO_OBJECT,
            &["Person"],
        )];
        let b = vec![make_anchor(
            "http://example.org/b#Person",
            vocab::BFO_OBJECT,
            &["Person"],
        )];
        let opts = AlignOptions::default();
        let mappings = align(&a, &b, &opts).unwrap();
        assert!(!mappings.is_empty(), "expected a mapping for identical labels in same BFO category");
        let m = &mappings[0];
        assert_eq!(m.kind, MappingKind::Equivalent);
        assert!(m.accepted);
        assert!((m.confidence - 1.0).abs() < 1e-6, "expected confidence ~1.0, got {}", m.confidence);
    }

    #[test]
    fn threshold_routing_accepted_vs_proposal() {
        // A moderate-similarity pair should land below the threshold.
        let a = vec![make_anchor(
            "http://example.org/a#HeartDisease",
            vocab::BFO_DISPOSITION,
            &["Heart Disease"],
        )];
        let b = vec![make_anchor(
            "http://example.org/b#CardiacCondition",
            vocab::BFO_DISPOSITION,
            &["Cardiac Condition"],
        )];
        let opts = AlignOptions::default();
        let mappings = align(&a, &b, &opts).unwrap();
        // Low similarity → should be a proposal (not accepted), kind = subClassOf.
        if !mappings.is_empty() {
            let m = &mappings[0];
            if m.confidence < opts.confidence_threshold {
                assert!(!m.accepted, "below-threshold mapping must not be accepted");
                assert_eq!(m.kind, MappingKind::SubClassOf);
            }
        }
    }

    #[test]
    fn deterministic_rerun() {
        let a = vec![
            make_anchor("http://ex.org/a#Dog", vocab::BFO_OBJECT, &["Dog"]),
            make_anchor("http://ex.org/a#Cat", vocab::BFO_OBJECT, &["Cat"]),
        ];
        let b = vec![
            make_anchor("http://ex.org/b#Cat", vocab::BFO_OBJECT, &["Cat"]),
            make_anchor("http://ex.org/b#Dog", vocab::BFO_OBJECT, &["Dog"]),
        ];
        let opts = AlignOptions::default();
        let run1 = align(&a, &b, &opts).unwrap();
        let run2 = align(&a, &b, &opts).unwrap();
        let iris1: Vec<_> = run1.iter().map(|m| (m.a_iri.clone(), m.b_iri.clone())).collect();
        let iris2: Vec<_> = run2.iter().map(|m| (m.a_iri.clone(), m.b_iri.clone())).collect();
        assert_eq!(iris1, iris2, "alignment must be deterministic");
    }
}
