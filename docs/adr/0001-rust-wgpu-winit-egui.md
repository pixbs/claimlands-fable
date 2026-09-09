# ADR 0001: Rust with wgpu, winit and egui

Date: 2026-09-09. Status: accepted.

## Context
The game targets iOS, Android and the web from one codebase, must render the prototype's pixel art
exactly, and is written mostly by autonomous agents, so compile-time safety and a small dependency
surface matter more than editor tooling. The prototype's geometry was written renderer-agnostic and
annotated "portable to wgpu/Rust later".

## Decision
Rust (stable, pinned in `rust-toolchain.toml`). `wgpu` for the GPU (Metal, Vulkan, WebGPU with WebGL2
fallback), `winit` for windows and input, `egui` via `egui-wgpu`/`egui-winit` for HUD and menus.
Targets: `wasm32-unknown-unknown`, `aarch64-linux-android`, `aarch64-apple-ios`; no desktop binary.
Pure crates are unit-tested on the CI host.

## Consequences
One WGSL shader set and one input model everywhere. Full control of the presentation pipeline, which
the pixel-exact requirement needs. Mobile packaging is plain Xcode and Gradle projects in
`platforms/`. Dependency versions are pinned and upgraded deliberately (`deps` label); an upstream
break is handled by a vendored patch (`vendor/`) rather than a downgrade.

## Alternatives
Bevy: its PBR and colour pipeline fight the raw-value look, ECS scheduling is a common source of
non-deterministic agent bugs, wasm is four times larger, and releases break APIs every few months.
TypeScript with three.js and Capacitor: fastest port but far from the hardware and without
cross-platform determinism. C++: no memory safety for unattended agents.
