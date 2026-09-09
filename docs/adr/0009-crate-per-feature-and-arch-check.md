# ADR 0009: Crate per feature, dependency allow-list

Date: 2026-09-09. Status: accepted.

## Context
Parallel agents collide on shared files, and a feature that reaches into another's internals is how
regressions spread. Cargo already enforces that a crate can only use what it declares.

## Decision
Features live in their own crates under `crates/` (workspace `members = ["crates/*"]`, so adding one
touches no shared file). Public APIs are documented in each crate's README with fixed sections.
An allow-list in `xtask/src/lint.rs`, mirrored in `docs/architecture.md`, states which internal
crates each crate may depend on and bans engine crates from the core; `cargo xtask lint-repo` fails
on any other edge. New crates are scaffolded by `cargo xtask new-crate`.

## Consequences
The dependency graph is a reviewed artefact; an "innocent" import that crosses a layer is a CI
failure, not a code-review catch. More `Cargo.toml` files and READMEs to maintain, which the
scaffold and the README-section lint keep uniform.

## Alternatives
Modules in one crate: no enforced boundaries. Feature flags: combinatorial builds and untested
combinations.
