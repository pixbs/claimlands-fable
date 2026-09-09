# ADR 0007: One-line level string

Date: 2026-09-09. Status: accepted.

## Context
Levels must be authored by a simple editor, stored in the campaign, shared by players later, and
reproduce the same map forever. A hex sphere has no natural 2D coordinates, and the generated world
is deterministic, so a level is a small diff over a seed.

## Decision
`docs/design/level-format.md`: a versioned, `;`-separated, URL-safe line with scalar keys and a
`tiles=` diff list of `<id><letters>` entries whose letter classes have disjoint alphabets. Tile ids
are hex-sphere indices, pinned by fixtures. Parsing is strict; serialisation is canonical; a round
trip is the identity (proptest).

## Consequences
Levels fit in a URL, a QR code or a chat message. The editor only needs to emit entries. Any future
key is additive under the same version; a breaking change bumps `CL1`. Ids depend on the geodesic
build order, so that order is part of the public contract (ADR 0004 pins it).

## Alternatives
JSON: verbose and not shareable by hand. Binary: opaque to review and to issues. Face/row/column
coordinates: exact but no simpler than ids for a sphere and harder to read in a diff.
