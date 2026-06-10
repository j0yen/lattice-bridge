//! BFO-anchor step: for each class in an ontology, resolve its BFO upper category.
//!
//! We parse the ontology's SubClassOf chain upward until we hit a known BFO IRI,
//! then record the category. This is the pruning step: only classes that share the
//! same BFO upper category are mapping candidates.

use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use horned_owl::model::{ClassExpression, Component};
use horned_owl::ontology::set::SetOntology;

use crate::error::BridgeError;
use crate::vocab;

/// The resolved BFO upper category for a class.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BfoCategory(pub String);

impl BfoCategory {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn label(&self) -> &str {
        vocab::local_name(&self.0)
    }
}

/// Per-class BFO anchor information.
#[derive(Debug, Clone)]
pub struct BfoAnchor {
    /// The class IRI.
    pub class_iri: String,
    /// The BFO upper category IRI this class falls under (or `owl:Thing` if unknown).
    pub category: BfoCategory,
    /// Labels extracted from annotation assertions (rdfs:label, synonyms).
    pub labels: Vec<String>,
    /// Definition text if present.
    pub definition: Option<String>,
    /// Direct superclass IRIs (named classes only, within this ontology).
    pub parents: Vec<String>,
}

/// Known BFO IRIs ranked from most-specific to broadest. We walk up the hierarchy
/// and take the first (deepest) BFO ancestor we find.
static BFO_CATEGORIES: &[&str] = &[
    vocab::BFO_OBJECT,
    vocab::BFO_OBJECT_AGGREGATE,
    vocab::BFO_SITE,
    vocab::BFO_MATERIAL_ENTITY,
    vocab::BFO_FUNCTION,
    vocab::BFO_ROLE,
    vocab::BFO_DISPOSITION,
    vocab::BFO_QUALITY,
    vocab::BFO_PROCESS,
    vocab::BFO_INDEPENDENT_CONTINUANT,
    vocab::BFO_SPECIFICALLY_DEPENDENT,
    vocab::BFO_GENERICALLY_DEPENDENT,
    vocab::BFO_CONTINUANT,
    vocab::BFO_OCCURRENT,
    vocab::BFO_ENTITY,
];

fn is_bfo(iri: &str) -> bool {
    iri.starts_with(vocab::BFO) || BFO_CATEGORIES.contains(&iri)
}

/// Extract a class IRI from a ClassExpression if it is a named class.
fn named_class_iri(ce: &ClassExpression<String>) -> Option<String> {
    match ce {
        ClassExpression::Class(c) => Some(String::from(&c.0)),
        _ => None,
    }
}

/// Parse annotation assertions to extract labels and definitions for each class.
fn extract_annotations(
    ont: &SetOntology<String>,
) -> HashMap<String, (Vec<String>, Option<String>)> {
    let mut map: HashMap<String, (Vec<String>, Option<String>)> = HashMap::new();

    for ac in ont.iter() {
        if let Component::AnnotationAssertion(aa) = &ac.component {
            // subject must be a simple IRI (not a blank node)
            let subject_iri = match &aa.subject {
                horned_owl::model::AnnotationSubject::IRI(iri) => iri.clone(),
                _ => continue,
            };
            let prop_iri: &str = &aa.ann.ap.0;
            let value = match &aa.ann.av {
                horned_owl::model::AnnotationValue::Literal(lit) => lit.literal().to_string(),
                horned_owl::model::AnnotationValue::IRI(iri) => iri.to_string(),
            };
            let entry = map.entry(subject_iri.to_string()).or_default();
            match prop_iri {
                vocab::RDFS_LABEL
                | vocab::OBO_EXACT_SYNONYM
                | vocab::OBO_BROAD_SYNONYM
                | vocab::OBO_NARROW_SYNONYM => {
                    entry.0.push(value);
                }
                vocab::OBO_DEFINITION => {
                    entry.1 = Some(value);
                }
                _ => {}
            }
        }
    }

    map
}

/// Build the direct-parent map (named sub → named sups) from SubClassOf axioms.
fn extract_parents(ont: &SetOntology<String>) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for ac in ont.iter() {
        if let Component::SubClassOf(sc) = &ac.component {
            if let (Some(sub), Some(sup)) = (
                named_class_iri(&sc.sub),
                named_class_iri(&sc.sup),
            ) {
                map.entry(sub).or_default().push(sup);
            }
        }
    }

    map
}

/// Walk the parent chain upward from `start_iri`, collecting visited class IRIs
/// until we hit a BFO IRI or exhaust the chain. Returns the set of ancestors.
fn ancestors(start: &str, parents: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(cur) = stack.pop() {
        if visited.contains(&cur) {
            continue;
        }
        visited.insert(cur.clone());
        if let Some(pars) = parents.get(&cur) {
            for p in pars {
                if !visited.contains(p) {
                    stack.push(p.clone());
                }
            }
        }
    }
    visited
}

/// Find the deepest (most specific) BFO category in an ancestor set.
fn deepest_bfo(anc: &HashSet<String>) -> Option<BfoCategory> {
    for bfo in BFO_CATEGORIES {
        if anc.contains(*bfo) {
            return Some(BfoCategory(bfo.to_string()));
        }
    }
    None
}

/// Load an ontology from an OWL/XML reader and compute BFO anchors for all classes.
///
/// Returns a vec of `BfoAnchor` — one per declared class.
pub fn load_anchors<R: BufRead>(reader: R) -> Result<Vec<BfoAnchor>, BridgeError> {
    let config = horned_owl::io::ParserConfiguration::default();
    let (ont, _prefixes): (SetOntology<String>, _) =
        horned_owl::io::owx::reader::read(&mut reader, config)
            .map_err(|e| BridgeError::OwlParse(e.to_string()))?;

    let annotations = extract_annotations(&ont);
    let parents = extract_parents(&ont);

    let mut declared: Vec<String> = Vec::new();
    for ac in ont.iter() {
        if let Component::DeclareClass(dc) = &ac.component {
            let iri = dc.0 .0.clone();
            let iri_str = iri.to_string();
            if !is_bfo(&iri_str) && iri_str != vocab::OWL_THING {
                declared.push(iri_str);
            }
        }
    }
    declared.sort();
    declared.dedup();

    let mut anchors = Vec::with_capacity(declared.len());
    for class_iri in declared {
        let anc = ancestors(&class_iri, &parents);
        let category = deepest_bfo(&anc)
            .unwrap_or_else(|| BfoCategory(vocab::OWL_THING.to_string()));

        let (labels, definition) = annotations
            .get(&class_iri)
            .cloned()
            .unwrap_or_default();

        // Fall back to local name as a label if nothing was annotated.
        let labels = if labels.is_empty() {
            vec![vocab::local_name(&class_iri).to_string()]
        } else {
            labels
        };

        let par_list = parents.get(&class_iri).cloned().unwrap_or_default();

        anchors.push(BfoAnchor {
            class_iri,
            category,
            labels,
            definition,
            parents: par_list,
        });
    }

    Ok(anchors)
}
