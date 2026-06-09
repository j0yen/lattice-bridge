//! lattice-bridge CLI: compute cross-ontology bridge axioms for BFO-grounded ontologies.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use lattice_bridge::{
    align, write_bridge_owl, write_proposals_jsonl, AlignOptions,
    anchor::load_anchors,
    output::print_proposals,
    error::BridgeError,
};

#[derive(Parser)]
#[command(name = "lattice-bridge", version, about = "Cross-ontology bridge axiom generator for BFO-grounded ontologies")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Align two BFO-grounded ontologies and emit bridge axioms.
    Align {
        /// Path or IRI of ontology A (OWL/XML).
        #[arg(long)]
        a: String,

        /// Path or IRI of ontology B (OWL/XML).
        #[arg(long)]
        b: String,

        /// Output OWL file for accepted bridge axioms.
        #[arg(long, default_value = "bridge.owl")]
        out: PathBuf,

        /// Proposals JSONL file (all candidates including low-confidence).
        #[arg(long, default_value = "bridge.proposals.jsonl")]
        proposals: PathBuf,

        /// Confidence threshold [0, 1]: mappings >= threshold go to --out, others to --proposals.
        #[arg(long, default_value_t = 0.85)]
        threshold: f64,

        /// Resolve --a and --b via `lattice-registry path`.
        #[arg(long)]
        from_registry: bool,

        /// IRI for the output bridge ontology.
        #[arg(long, default_value = "http://example.org/bridge")]
        bridge_iri: String,
    },

    /// Review a proposals JSONL file.
    Review {
        /// Path to bridge.proposals.jsonl.
        proposals: PathBuf,
    },
}

fn resolve_path(spec: &str, from_registry: bool) -> Result<PathBuf, BridgeError> {
    if from_registry {
        let output = std::process::Command::new("lattice-registry")
            .args(["path", spec])
            .output()
            .map_err(|e| BridgeError::Registry(format!("lattice-registry not found: {e}")))?;
        if !output.status.success() {
            return Err(BridgeError::Registry(format!(
                "lattice-registry path {spec}: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path))
    } else {
        Ok(PathBuf::from(spec))
    }
}

fn run() -> Result<(), BridgeError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Align {
            a,
            b,
            out,
            proposals,
            threshold,
            from_registry,
            bridge_iri,
        } => {
            let path_a = resolve_path(&a, from_registry)?;
            let path_b = resolve_path(&b, from_registry)?;

            eprintln!("Loading ontology A: {}", path_a.display());
            let a_anchors = {
                let f = File::open(&path_a)?;
                load_anchors(BufReader::new(f))?
            };
            eprintln!("  {} classes with BFO anchors", a_anchors.len());

            eprintln!("Loading ontology B: {}", path_b.display());
            let b_anchors = {
                let f = File::open(&path_b)?;
                load_anchors(BufReader::new(f))?
            };
            eprintln!("  {} classes with BFO anchors", b_anchors.len());

            let opts = AlignOptions {
                confidence_threshold: threshold,
                ..AlignOptions::default()
            };

            eprintln!("Aligning (threshold={threshold:.2})…");
            let mappings = align(&a_anchors, &b_anchors, &opts)?;
            let accepted = mappings.iter().filter(|m| m.accepted).count();
            let proposals_count = mappings.len() - accepted;
            eprintln!(
                "  {} mappings: {} accepted → bridge.owl, {} proposals → jsonl",
                mappings.len(),
                accepted,
                proposals_count
            );

            // Write OWL bridge file.
            {
                let f = File::create(&out)?;
                let mut w = BufWriter::new(f);
                write_bridge_owl(&mut w, &mappings, &bridge_iri)?;
            }
            eprintln!("  Wrote bridge axioms: {}", out.display());

            // Write proposals JSONL.
            {
                let f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&proposals)?;
                let mut w = BufWriter::new(f);
                write_proposals_jsonl(&mut w, &mappings)?;
            }
            eprintln!("  Wrote proposals: {}", proposals.display());
        }

        Commands::Review { proposals } => {
            let content = std::fs::read_to_string(&proposals)?;
            let mut mappings = Vec::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let m: lattice_bridge::matcher::Mapping = serde_json::from_str(line)?;
                mappings.push(m);
            }
            print_proposals(&mappings);
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
