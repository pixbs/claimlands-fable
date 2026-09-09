# Style

## Code

- Rust 2024, `rustfmt.toml`, clippy pedantic with the allow-list in `Cargo.toml`; warnings are errors.
- Public items have a doc comment that states what the item is for, not how it is implemented.
- Comments explain *why*: the reason for a value, an invariant, a non-obvious consequence. The
  prototype's constant comments are the model, e.g. "seven sides, not six: an odd count keeps the
  disc from lining up with the hex grid". Restating the code is noise.
- Ports keep the prototype's expressions and evaluation order; a deliberate deviation is commented
  with the reason and covered by a fixture or test.
- Constants are named and documented once, at the top of the module that owns them; no magic numbers
  in bodies.
- Errors are typed (`thiserror`); `panic`/`expect` only for programmer errors with a message that
  names the broken invariant.
- Tests: names describe the behaviour; assertions carry the input in the message.
- No `unsafe` outside `cl-render`/`cl-app` platform glue, and there only with a comment on the invariant.

## Prose (READMEs, docs, ADRs, issues, PRs)

- Length is whatever the content needs. Padding is the failure, not length.
- One idea per sentence. State facts, decisions, commands, invariants. Say why, not what, when the
  code already says what.
- Tables for anything enumerable; lists for parallel items; prose for a line of argument.
- No meta text that announces what the page is about, no hedging, no marketing adjectives, no restating the
  heading. `cargo xtask lint-repo` fails on the filler phrases listed in `xtask/src/lint.rs`, on empty
  sections, and on crate READMEs or ADRs missing their required sections.
- Reference things by path or id: `crates/cl-rules/src/apply.rs`, `R-ECO-03`, `hex-planet.html 1101–1130`.
- Review question for every paragraph: would a reader lose a fact if it were deleted?

## Naming

- Crates `cl-<feature>`; modules by responsibility, not by type (`economy.rs`, not `helpers.rs`).
- Rule ids `R-<GROUP>-<NN>`: `TURN`, `ECO`, `BLD`, `FOR`, `UNIT`, `CAP`, `VIC`, `STAT`.
- Fixture files `<subject>-n<frequency>-s<seed>.<ext>`.
- Branches `<type>/<issue>-<slug>`; PR titles `type(scope): summary`.
