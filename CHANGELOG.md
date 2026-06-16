# Changelog

## v0.2.0 — 2026-06-15

Fix: tolerate `owl:versionIRI` in OWL/XML parser; all OBO Foundry ontologies now parse cleanly.

- `load_anchors` now preprocesses the input XML before passing it to horned-owl, stripping
  `<owl:versionIRI rdf:resource="…"/>` which horned-owl 1.x rejects as an unexpected tag
  even though OWL 2 §3.1 requires parsers to accept it.
- The extracted versionIRI is returned as `OntologyMeta::version_iri` (new public type).
- New public function `preprocess_owl_xml(xml: &str) -> (String, OntologyMeta)`.
- `load_anchors` signature changed: now returns `Result<(Vec<BfoAnchor>, OntologyMeta), BridgeError>`.
- CLI (`lattice-bridge align`) logs the versionIRI when present.
- Tests: two new integration tests covering the versionIRI fixture and a regression against
  cached OBO Foundry OWL files (BFO, IAO, RO, COB, SWO).

## v0.1.0 — 2026-06-14

Initial release: cross-ontology bridge axiom generator for BFO-grounded ontologies.
