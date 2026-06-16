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

/// Metadata extracted from the OWL ontology header.
#[derive(Debug, Clone, Default)]
pub struct OntologyMeta {
    /// The `owl:versionIRI` value from the OWL/XML `<owl:Ontology>` header, if present.
    pub version_iri: Option<String>,
}

/// Preprocess OWL/XML: extract the `owl:versionIRI` IRI and strip version-related elements.
///
/// horned-owl 1.x rejects several OWL 2 header elements that appear inside `<owl:Ontology>`
/// in real-world ontologies (especially OBO Foundry RDF/XML files):
///
/// - `<owl:versionIRI rdf:resource="…"/>` — OWL 2 §3.1 standard; horned-owl rejects it.
/// - `<owl:versionInfo>text</owl:versionInfo>` — OWL 2 annotation; horned-owl rejects it
///   when it appears as a bare element inside `<Ontology>` rather than as an
///   `<AnnotationAssertion>` axiom.
///
/// This function strips both before passing to horned-owl's OWX reader.
/// The versionIRI IRI is captured in `OntologyMeta` for callers that need it.
pub fn preprocess_owl_xml(xml: &str) -> (String, OntologyMeta) {
    let mut version_iri: Option<String> = None;

    let cleaned = {
        let mut result = xml.to_string();

        // ── Strip <owl:versionIRI .../> and <owl:versionIRI ...></owl:versionIRI> ──
        // Both self-closing and paired-tag forms appear in the wild.
        let tag_open = "<owl:versionIRI";
        let mut search_from = 0;
        while let Some(start) = result[search_from..].find(tag_open) {
            let abs_start = search_from + start;
            if let Some(end_rel) = result[abs_start..].find("/>") {
                let abs_end = abs_start + end_rel + 2; // include "/>"
                let tag_content = &result[abs_start..abs_end];
                if version_iri.is_none() {
                    if let Some(iri) = extract_rdf_resource(tag_content) {
                        version_iri = Some(iri);
                    }
                }
                result.replace_range(abs_start..abs_end, "");
                // Don't advance search_from — the string has shifted.
            } else {
                let close_tag = "</owl:versionIRI>";
                if let Some(close_rel) = result[abs_start..].find(close_tag) {
                    let abs_end = abs_start + close_rel + close_tag.len();
                    let tag_content = &result[abs_start..abs_end];
                    if version_iri.is_none() {
                        if let Some(iri) = extract_rdf_resource(tag_content) {
                            version_iri = Some(iri);
                        }
                    }
                    result.replace_range(abs_start..abs_end, "");
                } else {
                    // Malformed — skip past the opening to avoid infinite loop.
                    search_from = abs_start + tag_open.len();
                }
            }
        }

        // ── Strip <owl:versionInfo>...</owl:versionInfo> ──
        // OBO Foundry files often include this text-content element in the ontology
        // header (e.g. `<owl:versionInfo>2026-03-30</owl:versionInfo>`).
        // horned-owl's OWX reader rejects it with "Unexpected end tag: expected versionInfo".
        strip_paired_tag(&mut result, "owl:versionInfo");

        result
    };

    (cleaned, OntologyMeta { version_iri })
}

/// Remove all occurrences of `<tag>…</tag>` from `xml`, including self-closing `<tag … />`.
/// `tag` should be the qualified name without angle brackets (e.g. `"owl:versionInfo"`).
fn strip_paired_tag(xml: &mut String, tag: &str) {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    loop {
        let Some(start) = xml.find(&open) else { break };
        // Self-closing variant: <tag ... />
        if let Some(sc_rel) = xml[start..].find("/>") {
            // Make sure there's no '>' before the '/>' that would indicate a paired tag.
            let gt_rel = xml[start..].find('>').unwrap_or(sc_rel + 1);
            if sc_rel < gt_rel {
                let end = start + sc_rel + 2;
                xml.replace_range(start..end, "");
                continue;
            }
        }
        // Paired variant: <tag ...>...</tag>
        if let Some(close_rel) = xml[start..].find(&close) {
            let end = start + close_rel + close.len();
            xml.replace_range(start..end, "");
        } else {
            // Malformed — bail out to avoid infinite loop.
            break;
        }
    }
}

/// Extract the value of the `rdf:resource` attribute from a tag string.
fn extract_rdf_resource(tag: &str) -> Option<String> {
    // Look for rdf:resource="VALUE" or rdf:resource='VALUE'
    let key = "rdf:resource=";
    let pos = tag.find(key)?;
    let rest = &tag[pos + key.len()..];
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[1..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
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
                horned_owl::model::AnnotationValue::AnonymousIndividual(_) => continue,
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
/// Returns a vec of `BfoAnchor` — one per declared class — plus `OntologyMeta` containing
/// the `owl:versionIRI` extracted from the OWL header (if present).
///
/// This function preprocesses the XML to strip `<owl:versionIRI …/>` before passing it to
/// horned-owl, which rejects that element as an unexpected tag even though OWL 2 §3.1
/// requires parsers to tolerate it.
pub fn load_anchors<R: BufRead>(mut reader: R) -> Result<(Vec<BfoAnchor>, OntologyMeta), BridgeError> {
    // Read the entire XML into a string so we can preprocess it.
    let mut xml = String::new();
    reader
        .read_to_string(&mut xml)
        .map_err(BridgeError::Io)?;

    let (cleaned_xml, meta) = preprocess_owl_xml(&xml);

    let config = horned_owl::io::ParserConfiguration::default();
    let mut cursor = std::io::BufReader::new(std::io::Cursor::new(cleaned_xml.into_bytes()));
    let (ont, _prefixes): (SetOntology<String>, _) =
        horned_owl::io::owx::reader::read(&mut cursor, config)
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

    Ok((anchors, meta))
}
