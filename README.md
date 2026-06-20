# lattice-bridge

Given two OWL ontologies that both anchor to BFO, it proposes the class mappings between them — which class in A is equivalent to, or a subclass of, which class in B.

## Why it exists

Two ontologies built on the same upper ontology should be alignable, but the alignment is tedious to do by hand and dangerous to do blindly. BFO gives the alignment a spine: every class lands under one of BFO's upper categories — material entity, process, quality, role, and the rest — and a material entity can never map to a process. `lattice-bridge` uses that. It groups the classes of each ontology by their BFO upper category and only ever compares classes within the same bucket. The combinatorial explosion of "every class against every class" collapses to "every class against the few that could possibly match," and a whole class of nonsense mappings is ruled out before scoring begins.

## Install

```
cargo install --path .
```

Requires Rust ≥ 1.88. Input ontologies are OWL/XML.

## Quickstart

```
lattice-bridge align --a iao.owl --b cob.owl --threshold 0.85
```

This prints progress to stderr — how many BFO-anchored classes each ontology has, how many mappings cleared the threshold — and writes two files:

- `bridge.owl` — the accepted mappings (confidence ≥ threshold), as OWL bridge axioms ready to load.
- `bridge.proposals.jsonl` — every candidate, including the ones below threshold, one JSON object per line for review.

Then review the borderline cases:

```
lattice-bridge review bridge.proposals.jsonl
```

If you keep your ontologies in [`lattice-registry`](https://github.com/j0yen/lattice-registry), pass short names and let the registry resolve the paths:

```
lattice-bridge align --from-registry --a iao --b cob
```

## How it works

The pipeline is four steps: anchor, prune, score, classify.

1. **Anchor.** For each class, walk its `SubClassOf` chain upward until it hits a known BFO IRI, and record that upper category. Labels (`rdfs:label`, OBO exact/broad/narrow synonyms) and the `IAO:definition` are collected along the way.
2. **Prune.** Group classes by BFO category. Only same-category pairs are ever scored — this is where the BFO grounding earns its keep.
3. **Score.** Each pair gets a lexical score: `0.7 × normalized edit distance + 0.3 × token-overlap Jaccard`, taken as the best match over every label/synonym pairing.
4. **Classify and threshold.** Confidence is `lex_weight × lexical + (1 − lex_weight) × structural`. Mappings at or above `--threshold` go to `bridge.owl`; the rest go to the proposals file.

A note on the scoring weights, stated plainly because it changes how you should read the output: the structural score is currently a stub that always returns 0.0, and the default `lex_weight` is 0.9. So today the confidence is, in effect, lexical similarity scaled by 0.9 — a perfect label match scores 0.9 and clears the default 0.85 threshold. A structural pass (do the parents of A map to the parents of B?) is designed into the interface but not yet implemented. Treat the current output as label-driven alignment, and review the proposals accordingly.

## Where it fits

`lattice-bridge` and [`lattice-registry`](https://github.com/j0yen/lattice-registry) are a pair: the registry is the local catalog of BFO-grounded ontologies, and `lattice-bridge --from-registry` resolves its `--a`/`--b` arguments through `lattice-registry path`.

## Status

Working for the alignment it does today: anchor, BFO-category prune, lexical scoring, OWL/JSONL output. The OWL/XML parser (built on horned-owl) tolerates `owl:versionIRI` headers, which it rejected before v0.2.0, so the OBO Foundry ontologies (BFO, IAO, RO, COB, SWO) parse cleanly. The structural-scoring pass is the next iteration.

## License

MIT OR Apache-2.0
