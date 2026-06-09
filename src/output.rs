//! Output serialization: OWL/XML bridge axioms and JSONL proposals.

use std::io::Write;

use crate::error::BridgeError;
use crate::matcher::{Mapping, MappingKind};

/// Write accepted mappings as OWL/XML bridge axioms to `out`.
///
/// Emits an OWL ontology document with `EquivalentClasses` axioms for
/// `MappingKind::Equivalent` and `SubClassOf` axioms for `MappingKind::SubClassOf`.
/// Only mappings with `accepted == true` are included.
pub fn write_bridge_owl<W: Write>(
    out: &mut W,
    mappings: &[Mapping],
    onto_iri: &str,
) -> Result<(), BridgeError> {
    writeln!(
        out,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE Ontology []>
<Ontology
    xmlns="http://www.w3.org/2002/07/owl#"
    xml:base="{onto_iri}"
    ontologyIRI="{onto_iri}">
  <Prefix name="owl" IRI="http://www.w3.org/2002/07/owl#"/>
  <Prefix name="rdf" IRI="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>
  <Prefix name="rdfs" IRI="http://www.w3.org/2000/01/rdf-schema#"/>
  <Prefix name="xsd" IRI="http://www.w3.org/2001/XMLSchema#"/>"#
    )?;

    for m in mappings.iter().filter(|m| m.accepted) {
        match m.kind {
            MappingKind::Equivalent => {
                writeln!(
                    out,
                    r#"  <!-- confidence={:.4} bfo_category={} -->
  <EquivalentClasses>
    <Class IRI="{a}"/>
    <Class IRI="{b}"/>
  </EquivalentClasses>"#,
                    m.confidence,
                    m.bfo_category,
                    a = m.a_iri,
                    b = m.b_iri,
                )?;
            }
            MappingKind::SubClassOf => {
                writeln!(
                    out,
                    r#"  <!-- confidence={:.4} bfo_category={} -->
  <SubClassOf>
    <Class IRI="{sub}"/>
    <Class IRI="{sup}"/>
  </SubClassOf>"#,
                    m.confidence,
                    m.bfo_category,
                    sub = m.a_iri,
                    sup = m.b_iri,
                )?;
            }
        }
    }

    writeln!(out, "</Ontology>")?;
    Ok(())
}

/// Write all mappings (accepted + proposals) as JSONL to `out`.
///
/// Each line is a JSON object representing one `Mapping`.
pub fn write_proposals_jsonl<W: Write>(
    out: &mut W,
    mappings: &[Mapping],
) -> Result<(), BridgeError> {
    for m in mappings {
        let line = serde_json::to_string(m)?;
        writeln!(out, "{line}")?;
    }
    Ok(())
}

/// Pretty-print proposals to stdout (used by `lattice-bridge review`).
pub fn print_proposals(mappings: &[Mapping]) {
    if mappings.is_empty() {
        println!("No proposals.");
        return;
    }
    println!(
        "{:<8} {:<12} {:<40} {:<40} {:<16} {:<6} {}",
        "ACCEPT", "KIND", "A", "B", "BFO", "CONF", "EVIDENCE"
    );
    println!("{}", "-".repeat(140));
    for m in mappings {
        let accept = if m.accepted { "YES" } else { "review" };
        let bfo_short = m
            .bfo_category
            .rsplit(['/', '#'])
            .next()
            .unwrap_or(&m.bfo_category);
        let evidence = format!(
            "lex={:.3} struct={:.3}",
            m.lexical_score, m.structural_score
        );
        println!(
            "{:<8} {:<12} {:<40} {:<40} {:<16} {:<6.3} {}",
            accept,
            m.kind.to_string(),
            truncate(&m.a_label, 40),
            truncate(&m.b_label, 40),
            truncate(bfo_short, 16),
            m.confidence,
            evidence,
        );
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Mapping, MappingKind};

    fn dummy_mapping(accepted: bool, kind: MappingKind, conf: f64) -> Mapping {
        Mapping {
            a_iri: "http://a.org/A".to_string(),
            b_iri: "http://b.org/B".to_string(),
            bfo_category: "http://purl.obolibrary.org/obo/BFO_0000030".to_string(),
            lexical_score: conf,
            structural_score: 0.0,
            confidence: conf,
            kind,
            a_label: "A".to_string(),
            b_label: "B".to_string(),
            accepted,
        }
    }

    #[test]
    fn owl_output_parses_back() {
        let m = dummy_mapping(true, MappingKind::Equivalent, 0.95);
        let mut buf = Vec::new();
        write_bridge_owl(&mut buf, &[m], "http://example.org/bridge").unwrap();
        let xml = String::from_utf8(buf).unwrap();
        assert!(xml.contains("<EquivalentClasses>"));
        assert!(xml.contains("http://a.org/A"));
        assert!(xml.contains("http://b.org/B"));
    }

    #[test]
    fn rejected_mapping_not_in_owl() {
        let m = dummy_mapping(false, MappingKind::SubClassOf, 0.5);
        let mut buf = Vec::new();
        write_bridge_owl(&mut buf, &[m], "http://example.org/bridge").unwrap();
        let xml = String::from_utf8(buf).unwrap();
        assert!(!xml.contains("<SubClassOf>"), "below-threshold must not appear in bridge.owl");
    }

    #[test]
    fn jsonl_roundtrip() {
        let m = dummy_mapping(true, MappingKind::Equivalent, 0.95);
        let mut buf = Vec::new();
        write_proposals_jsonl(&mut buf, &[m]).unwrap();
        let line = String::from_utf8(buf).unwrap();
        let parsed: Mapping = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed.a_iri, "http://a.org/A");
        assert_eq!(parsed.kind, MappingKind::Equivalent);
    }
}
