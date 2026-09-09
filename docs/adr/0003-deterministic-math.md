# ADR 0003: Deterministic math

Date: 2026-09-09. Status: accepted.

## Context
Worlds, replays, shared levels and future multiplayer all require that the same seed produces the
same planet on iOS, Android, the web and CI, and the fidelity contract requires it to equal the
prototype's output.

## Decision
Generation math is `f64` with the prototype's operation order, transcendental functions from the
`libm` crate, V8's compensated `hypot`, JavaScript `round`, `ToInt32` and `toFixed(6)` semantics
ported in `cl-noise::js`, `mulberry32` and `hash3i` as the only random and hash sources, and no
hash-ordered collections. `clippy.toml` bans `f64::sin`-style std calls and `HashMap`/`HashSet`.
Fixture tests compare bits.

## Consequences
The hex sphere, worldgen and mesh builders reproduce the prototype exactly today. A residue remains:
`libm`'s `sin`, `cos` and `pow` differ from V8's fdlibm in the last bit for about 1 % of inputs, which
only reaches `hash2` and two texture strips; an issue ports fdlibm to remove it. Serde fixtures need
`float_roundtrip`. Gameplay randomness uses `Mulberry32` as well, so replays reproduce AI turns.

## Alternatives
Tolerance-based comparison everywhere: hides real bugs behind noise. Fixed-point arithmetic: exact
but would change the prototype's visuals. Platform libm: fastest, but different bits per device.
