# vendor

Patched copies of upstream crates, applied through `[patch.crates-io]` in the root `Cargo.toml`.
Each entry states the upstream version, the change, and the condition for removing it. Nothing else
belongs here; vendoring is a last resort, never a way to fork behaviour.

| Crate | Upstream | Change | Remove when |
|---|---|---|---|
| `egui-winit` | 0.36.2 (crates.io, 2026-09-08) | `src/dropped_file.rs`: `bytes` is gated to non-wasm targets and `bytes_async` is provided for wasm32, matching the `egui::DroppedFile` trait's own cfg split. Without it egui-winit does not compile for `wasm32-unknown-unknown` (E0407/E0046). | an egui-winit release compiles for wasm32 unpatched (`cargo check -p cl-app --target wasm32-unknown-unknown` after removing the patch line) |
