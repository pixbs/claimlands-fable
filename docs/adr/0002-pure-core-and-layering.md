# ADR 0002: Pure core, ports and adapters

Date: 2026-09-09. Status: accepted.

## Context
Adding a feature must not break another, several agents work at once, and every rule needs a test
that runs in milliseconds. The prototype already separates pure geometry from three.js calls.

## Decision
Layers with a one-way dependency graph enforced by `cargo xtask lint-repo` (`docs/architecture.md`).
The rules are a pure state machine (`cl-rules::apply`) over an adjacency trait (`Board`); visuals are
pure functions from `WorldSnapshot` to `MeshData` and `RgbaImage`; only `cl-render`, `cl-ui` and
`cl-app` touch wgpu, egui and winit. Communication upward happens through returned `Event`s.

## Consequences
Rules are tested on a seven-tile graph; meshes are tested against fixtures without a GPU; the app is a
thin wiring layer. A crate's public API is its contract: changes need README updates and reviews.
Some duplication of plain types in `cl-model` is accepted to keep the core engine-free.

## Alternatives
An ECS with systems per feature: convenient, but ordering and shared mutable resources are where
agent-written code fails silently. A single crate with modules: fewer files, but no compiler-enforced
boundaries and constant merge conflicts in shared files.
