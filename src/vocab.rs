//! IRI constants for BFO, OWL, RDF, and common annotation properties.

pub const BFO: &str = "http://purl.obolibrary.org/obo/BFO_";
pub const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDFS_SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const OWL_EQUIV: &str = "http://www.w3.org/2002/07/owl#equivalentClass";

/// Annotation properties used to extract labels for lexical matching.
pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const OBO_EXACT_SYNONYM: &str =
    "http://www.geneontology.org/formats/oboInOwl#hasExactSynonym";
pub const OBO_BROAD_SYNONYM: &str =
    "http://www.geneontology.org/formats/oboInOwl#hasBroadSynonym";
pub const OBO_NARROW_SYNONYM: &str =
    "http://www.geneontology.org/formats/oboInOwl#hasNarrowSynonym";
pub const OBO_DEFINITION: &str = "http://purl.obolibrary.org/obo/IAO_0000115";

/// Well-known BFO 2.0 upper-category IRIs (the seven primary universals we anchor to).
pub const BFO_ENTITY: &str = "http://purl.obolibrary.org/obo/BFO_0000001";
pub const BFO_CONTINUANT: &str = "http://purl.obolibrary.org/obo/BFO_0000002";
pub const BFO_OCCURRENT: &str = "http://purl.obolibrary.org/obo/BFO_0000003";
pub const BFO_INDEPENDENT_CONTINUANT: &str = "http://purl.obolibrary.org/obo/BFO_0000004";
pub const BFO_SPECIFICALLY_DEPENDENT: &str = "http://purl.obolibrary.org/obo/BFO_0000020";
pub const BFO_GENERICALLY_DEPENDENT: &str = "http://purl.obolibrary.org/obo/BFO_0000031";
pub const BFO_MATERIAL_ENTITY: &str = "http://purl.obolibrary.org/obo/BFO_0000040";
pub const BFO_PROCESS: &str = "http://purl.obolibrary.org/obo/BFO_0000015";
pub const BFO_QUALITY: &str = "http://purl.obolibrary.org/obo/BFO_0000019";
pub const BFO_DISPOSITION: &str = "http://purl.obolibrary.org/obo/BFO_0000016";
pub const BFO_ROLE: &str = "http://purl.obolibrary.org/obo/BFO_0000023";
pub const BFO_FUNCTION: &str = "http://purl.obolibrary.org/obo/BFO_0000034";
pub const BFO_OBJECT: &str = "http://purl.obolibrary.org/obo/BFO_0000030";
pub const BFO_OBJECT_AGGREGATE: &str = "http://purl.obolibrary.org/obo/BFO_0000027";
pub const BFO_SITE: &str = "http://purl.obolibrary.org/obo/BFO_0000029";

/// Return the local fragment of an IRI (after last `/` or `#`).
pub fn local_name(iri: &str) -> &str {
    iri.rsplit(['/', '#']).next().unwrap_or(iri)
}
