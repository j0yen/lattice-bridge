//! Integration tests for lattice-bridge using small inline OWL/XML fixture ontologies.

use lattice_bridge::{
    align, write_bridge_owl, write_proposals_jsonl, AlignOptions,
    anchor::load_anchors,
    matcher::{Mapping, MappingKind},
};

/// A minimal OWL/XML ontology with BFO-anchored classes.
///
/// Declares:
/// - `ex:Person` subClassOf `BFO:0000030` (material entity / object)
/// - `ex:Animal` subClassOf `BFO:0000030`
/// - `ex:Fever`  subClassOf `BFO:0000019` (quality)
/// - `ex:Process` subClassOf `BFO:0000015` (process)
fn make_ontology_a() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<Ontology xmlns="http://www.w3.org/2002/07/owl#"
          xml:base="http://example.org/onto-a"
          ontologyIRI="http://example.org/onto-a">
  <Prefix name="owl" IRI="http://www.w3.org/2002/07/owl#"/>
  <Prefix name="rdf" IRI="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>
  <Prefix name="rdfs" IRI="http://www.w3.org/2000/01/rdf-schema#"/>
  <Prefix name="xsd" IRI="http://www.w3.org/2001/XMLSchema#"/>
  <Prefix name="obo" IRI="http://purl.obolibrary.org/obo/"/>

  <Declaration><Class IRI="http://example.org/onto-a#Person"/></Declaration>
  <Declaration><Class IRI="http://example.org/onto-a#Animal"/></Declaration>
  <Declaration><Class IRI="http://example.org/onto-a#Fever"/></Declaration>
  <Declaration><Class IRI="http://example.org/onto-a#HeatingProcess"/></Declaration>

  <SubClassOf>
    <Class IRI="http://example.org/onto-a#Person"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000030"/>
  </SubClassOf>
  <SubClassOf>
    <Class IRI="http://example.org/onto-a#Animal"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000030"/>
  </SubClassOf>
  <SubClassOf>
    <Class IRI="http://example.org/onto-a#Fever"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000019"/>
  </SubClassOf>
  <SubClassOf>
    <Class IRI="http://example.org/onto-a#HeatingProcess"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000015"/>
  </SubClassOf>

  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-a#Person</IRI>
    <Literal>Person</Literal>
  </AnnotationAssertion>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-a#Animal</IRI>
    <Literal>Animal</Literal>
  </AnnotationAssertion>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-a#Fever</IRI>
    <Literal>Fever</Literal>
  </AnnotationAssertion>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-a#HeatingProcess</IRI>
    <Literal>Heating Process</Literal>
  </AnnotationAssertion>
</Ontology>"#
}

/// Ontology B:
/// - `wo:Person`  subClassOf `BFO:0000030` (same category as A's Person)
/// - `wo:Warmth`  subClassOf `BFO:0000019` (quality — different label from A's Fever)
/// - `wo:Fever`   subClassOf `BFO:0000015` (PROCESS — same label as A's quality Fever!)
fn make_ontology_b() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<Ontology xmlns="http://www.w3.org/2002/07/owl#"
          xml:base="http://example.org/onto-b"
          ontologyIRI="http://example.org/onto-b">
  <Prefix name="owl" IRI="http://www.w3.org/2002/07/owl#"/>
  <Prefix name="rdf" IRI="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>
  <Prefix name="rdfs" IRI="http://www.w3.org/2000/01/rdf-schema#"/>
  <Prefix name="xsd" IRI="http://www.w3.org/2001/XMLSchema#"/>
  <Prefix name="obo" IRI="http://purl.obolibrary.org/obo/"/>

  <Declaration><Class IRI="http://example.org/onto-b#Person"/></Declaration>
  <Declaration><Class IRI="http://example.org/onto-b#Warmth"/></Declaration>
  <Declaration><Class IRI="http://example.org/onto-b#Fever"/></Declaration>

  <SubClassOf>
    <Class IRI="http://example.org/onto-b#Person"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000030"/>
  </SubClassOf>
  <SubClassOf>
    <Class IRI="http://example.org/onto-b#Warmth"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000019"/>
  </SubClassOf>
  <SubClassOf>
    <!-- Fever in B is a PROCESS, not a quality — same label, different BFO bucket -->
    <Class IRI="http://example.org/onto-b#Fever"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000015"/>
  </SubClassOf>

  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-b#Person</IRI>
    <Literal>Person</Literal>
  </AnnotationAssertion>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-b#Warmth</IRI>
    <Literal>Warmth</Literal>
  </AnnotationAssertion>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/onto-b#Fever</IRI>
    <Literal>Fever</Literal>
  </AnnotationAssertion>
</Ontology>"#
}

fn load(xml: &str) -> Vec<lattice_bridge::anchor::BfoAnchor> {
    let (anchors, _meta) = load_anchors(std::io::BufReader::new(xml.as_bytes())).expect("load_anchors failed");
    anchors
}

// ── AC1: align emits candidate mappings with required fields ─────────────────

#[test]
fn ac1_align_emits_candidate_with_required_fields() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();
    let mappings = align(&a, &b, &opts).unwrap();

    assert!(!mappings.is_empty(), "expected at least one mapping");
    for m in &mappings {
        // Must have a BFO category, a confidence in [0,1].
        assert!(!m.bfo_category.is_empty());
        assert!(m.confidence >= 0.0 && m.confidence <= 1.0);
    }
}

// ── AC2: BFO-anchor prune blocks cross-category mappings ────────────────────

#[test]
fn ac2_bfo_prune_blocks_fever_quality_to_fever_process() {
    // A's Fever is under BFO_quality (0000019); B's Fever is under BFO_process (0000015).
    // They share the label "Fever" but MUST NOT be mapped.
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();
    let mappings = align(&a, &b, &opts).unwrap();

    let cross = mappings.iter().find(|m| {
        m.a_iri.ends_with("#Fever") && m.b_iri.ends_with("#Fever")
    });
    assert!(
        cross.is_none(),
        "cross-BFO-category label collision must be pruned: got {:?}",
        cross
    );
}

// ── AC3: threshold routing — high confidence → OWL, low → proposals ─────────

#[test]
fn ac3_threshold_routing() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions {
        confidence_threshold: 0.85,
        ..Default::default()
    };
    let mappings = align(&a, &b, &opts).unwrap();

    for m in &mappings {
        if m.confidence >= 0.85 {
            assert!(m.accepted, "above-threshold mapping must be accepted");
            assert_eq!(m.kind, MappingKind::Equivalent);
        } else {
            assert!(!m.accepted, "below-threshold mapping must not be accepted");
            assert_eq!(m.kind, MappingKind::SubClassOf);
        }
    }

    // The identical "Person" pair must be accepted.
    let person_mapping = mappings.iter().find(|m| {
        m.a_iri.ends_with("#Person") && m.b_iri.ends_with("#Person")
    });
    assert!(
        person_mapping.is_some(),
        "Person↔Person mapping must exist"
    );
    let pm = person_mapping.unwrap();
    assert!(pm.accepted, "Person↔Person must be accepted (high confidence)");
}

// ── AC3/AC4: bridge.owl parses back via horned-owl ───────────────────────────

#[test]
fn ac4_bridge_owl_is_parseable() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();
    let mappings = align(&a, &b, &opts).unwrap();

    let mut buf = Vec::new();
    write_bridge_owl(&mut buf, &mappings, "http://example.org/bridge").unwrap();
    let xml = String::from_utf8(buf).unwrap();

    // Must be valid XML and contain the OWL Ontology element.
    assert!(xml.contains("<Ontology"));
    assert!(xml.contains("</Ontology>"));

    // Re-parse via horned-owl's OWL/XML reader to confirm it's valid OWL.
    use horned_owl::io::ParserConfiguration;
    use horned_owl::ontology::set::SetOntology;
    let config = ParserConfiguration::default();
    let result: Result<(SetOntology<String>, _), _> =
        horned_owl::io::owx::reader::read(&mut std::io::BufReader::new(xml.as_bytes()), config);
    assert!(result.is_ok(), "bridge.owl must be re-parseable by horned-owl: {:?}", result.err());
}

// ── AC6: deterministic re-run ────────────────────────────────────────────────

#[test]
fn ac6_deterministic_rerun() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();

    let run1 = align(&a, &b, &opts).unwrap();
    let run2 = align(&a, &b, &opts).unwrap();

    let key = |m: &Mapping| (m.a_iri.clone(), m.b_iri.clone(), m.confidence.to_bits());
    let k1: Vec<_> = run1.iter().map(key).collect();
    let k2: Vec<_> = run2.iter().map(key).collect();
    assert_eq!(k1, k2, "alignment must be deterministic across runs");
}

// ── AC7: known-good Person↔Person equivalence at high confidence ─────────────

#[test]
fn ac7_person_person_high_confidence_equivalence() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();
    let mappings = align(&a, &b, &opts).unwrap();

    let pm = mappings
        .iter()
        .find(|m| m.a_iri.ends_with("#Person") && m.b_iri.ends_with("#Person"))
        .expect("Person↔Person mapping must exist");

    assert_eq!(pm.kind, MappingKind::Equivalent);
    assert!(pm.accepted);
    assert!(
        pm.confidence >= 0.85,
        "Person↔Person confidence must be >= 0.85, got {}",
        pm.confidence
    );
}

// ── AC4-versionIRI: fixture with owl:versionIRI parses cleanly ───────────────

#[test]
fn ac4_version_iri_fixture_parses_without_error() {
    // Minimal OWL/XML with owl:versionIRI inside owl:Ontology — this is valid OWL 2 DL
    // but horned-owl 1.x rejects it. preprocess_owl_xml must strip it before parsing.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ontology xmlns="http://www.w3.org/2002/07/owl#"
          xml:base="http://example.org/versioned"
          ontologyIRI="http://example.org/versioned">
  <Prefix name="owl" IRI="http://www.w3.org/2002/07/owl#"/>
  <Prefix name="rdf" IRI="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>
  <Prefix name="rdfs" IRI="http://www.w3.org/2000/01/rdf-schema#"/>
  <Prefix name="obo" IRI="http://purl.obolibrary.org/obo/"/>
  <owl:versionIRI rdf:resource="http://example.org/v1"/>
  <Declaration><Class IRI="http://example.org/versioned#Widget"/></Declaration>
  <SubClassOf>
    <Class IRI="http://example.org/versioned#Widget"/>
    <Class IRI="http://purl.obolibrary.org/obo/BFO_0000030"/>
  </SubClassOf>
  <AnnotationAssertion>
    <AnnotationProperty IRI="http://www.w3.org/2000/01/rdf-schema#label"/>
    <IRI>http://example.org/versioned#Widget</IRI>
    <Literal>Widget</Literal>
  </AnnotationAssertion>
</Ontology>"#;

    let (anchors, meta) =
        load_anchors(std::io::BufReader::new(xml.as_bytes())).expect("load_anchors must not fail on versionIRI");

    // The versionIRI must be captured.
    assert_eq!(
        meta.version_iri.as_deref(),
        Some("http://example.org/v1"),
        "versionIRI must be extracted from the OWL header"
    );

    // The class must still be parsed correctly.
    assert_eq!(anchors.len(), 1, "expected 1 class (Widget)");
    assert!(
        anchors[0].class_iri.ends_with("#Widget"),
        "Widget class must be present"
    );
}

// ── AC4-regression: cached OBO Foundry OWL files parse without error ─────────

#[test]
fn ac4_regression_cached_obo_foundry_owls_parse() {
    let cache_dir = match std::env::var("HOME") {
        Ok(h) => std::path::PathBuf::from(h).join(".cache/lattice/registry/owls"),
        Err(_) => {
            eprintln!("(cached files absent, install test skipped — HOME not set)");
            return;
        }
    };

    let candidates = ["bfo.owl", "iao.owl", "ro.owl", "cob.owl", "swo.owl"];
    let mut found_any = false;

    for name in &candidates {
        let path = cache_dir.join(name);
        if !path.exists() {
            continue;
        }
        found_any = true;
        let f = std::fs::File::open(&path)
            .unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()));
        let result = load_anchors(std::io::BufReader::new(f));
        assert!(
            result.is_ok(),
            "{} must parse without error, got: {:?}",
            name,
            result.err()
        );
        let (anchors, _meta) = result.unwrap();
        // Note: bfo.owl itself contains the BFO hierarchy — those classes are filtered
        // out by is_bfo() since they ARE the BFO categories, not subclasses of them.
        // Non-BFO ontologies (iao, ro, cob, swo) will have non-BFO classes anchored
        // to BFO categories. We only assert parse-without-error here (the regression
        // AC), not a non-empty anchor count.
        eprintln!("  {name}: {} BFO-anchored classes parsed OK", anchors.len());
    }

    if !found_any {
        eprintln!("(cached files absent, install test skipped)");
    }
}

// ── AC5: --from-registry invokes lattice-registry path ───────────────────────
//
// We cannot rely on lattice-registry being installed, so we verify the error path:
// when --from-registry is passed and lattice-registry is absent (or returns a
// non-zero exit), the CLI exits non-zero with a registry error message.
// This confirms the flag is wired and the resolve_path call is reachable.
#[test]
fn ac5_from_registry_returns_error_when_registry_absent() {
    use std::process::Command;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let out = dir.path().join("bridge.owl");
    let proposals = dir.path().join("bridge.proposals.jsonl");

    // Run the binary with a non-existent ontology spec + --from-registry.
    // lattice-registry is either absent or will return non-zero for a bogus spec.
    // Either way the CLI must exit non-zero and mention the registry.
    let status = Command::new(env!("CARGO_BIN_EXE_lattice-bridge"))
        .args([
            "align",
            "--a", "no-such-ontology-id",
            "--b", "no-such-ontology-id-b",
            "--from-registry",
            "--out", out.to_str().unwrap(),
            "--proposals", proposals.to_str().unwrap(),
        ])
        .output()
        .expect("failed to spawn lattice-bridge binary");

    // Must fail — registry not available or returned error.
    assert!(
        !status.status.success(),
        "--from-registry with missing registry must exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&status.stderr);
    // The error must mention 'registry' (from BridgeError::Registry)
    assert!(
        stderr.contains("registry") || stderr.contains("lattice-registry"),
        "stderr must mention registry error, got: {stderr}"
    );
}

// ── AC8: proposals JSONL roundtrip ───────────────────────────────────────────

#[test]
fn ac8_proposals_jsonl_roundtrip() {
    let a = load(make_ontology_a());
    let b = load(make_ontology_b());
    let opts = AlignOptions::default();
    let mappings = align(&a, &b, &opts).unwrap();

    let mut buf = Vec::new();
    write_proposals_jsonl(&mut buf, &mappings).unwrap();
    let content = String::from_utf8(buf).unwrap();

    // Each line must parse as a Mapping.
    let parsed: Vec<Mapping> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Mapping>(l).expect("valid JSON line"))
        .collect();

    assert_eq!(
        parsed.len(),
        mappings.len(),
        "JSONL must contain same number of mappings"
    );

    // First entries should match (same sort order).
    if !parsed.is_empty() {
        assert_eq!(parsed[0].a_iri, mappings[0].a_iri);
        assert_eq!(parsed[0].b_iri, mappings[0].b_iri);
    }
}
