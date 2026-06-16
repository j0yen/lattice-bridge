//! lattice-bridge: cross-ontology bridge axiom generator for BFO-grounded ontologies.
//!
//! Given two OWL ontologies that both anchor to BFO upper categories, computes
//! candidate class mappings pruned by BFO category, scored by lexical + structural
//! similarity, and serialized as OWL bridge axioms or review proposals.

pub mod anchor;
pub mod error;
pub mod lexical;
pub mod matcher;
pub mod output;
pub mod structural;
pub mod vocab;

pub use anchor::{BfoAnchor, BfoCategory, OntologyMeta, preprocess_owl_xml};
pub use error::BridgeError;
pub use matcher::{align, AlignOptions, Mapping, MappingKind};
pub use output::{write_bridge_owl, write_proposals_jsonl};
