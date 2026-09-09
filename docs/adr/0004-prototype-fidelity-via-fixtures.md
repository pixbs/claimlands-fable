# ADR 0004: Prototype fidelity through extracted fixtures

Date: 2026-09-09. Status: accepted.

## Context
The visuals must look exactly like `hex-planet.html`. Reviewing screenshots by eye does not scale
across a swarm of agents porting sections in parallel, and the prototype computes every texture and
mesh in pure functions that can be observed.

## Decision
The prototype is frozen in `reference/` (its SHA-256 recorded in `fixtures/index.json`). A Node
harness runs it with recorder shims and writes every intermediate the port must match: hashes, tile
geometry, levels and cover, PNGs of every texture, and exact hashes of every mesh attribute. Each
crate replays its fixtures in tests; tolerances exist only where ADR 0003 documents a known residue.
Fixture changes require the `visual-change` label and a reason. The prototype is also served at
`/reference/` on every preview for A/B by eye, and a screenshot regression test follows in M1.

## Consequences
A port is done when its fixture test is green, which agents can verify without judgement. Bugs
reveal themselves as bit differences with inputs printed. The harness is a small permanent tool in
`reference/harness`; regenerating fixtures is a deliberate, reviewed act.

## Alternatives
Screenshot diffs only: coarse, environment-dependent, and useless for locating a bug. Trusting the
port by inspection: the failure mode this repository exists to prevent.
