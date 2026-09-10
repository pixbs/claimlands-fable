# reference

`hex-planet.html` is the visual prototype and the oracle for everything the game renders. It is
frozen: never edit it. Its SHA-256 is recorded in `fixtures/index.json`, and it is served at
`/reference/` on every preview so any change can be compared by eye at the same seed.

`harness/` runs the prototype's inline script in `node:vm` with stand-ins for the browser and for
three.js (`shims.mjs`), calls the builders directly, and writes `fixtures/` (`extract.mjs`).
`png.mjs` is a dependency-free PNG encoder.

```bash
cargo xtask fixtures          # node reference/harness/extract.mjs
cargo xtask fixtures --full   # also dumps complete arrays to reference/harness/out/ (gitignored)
```

The harness changes only to extract more; it never changes what the prototype computes. Fixture
diffs in a PR need the `visual-change` label and a reason. `docs/testing.md` lists what each fixture
directory holds and how tests compare it.

`designs/` holds [approved supplemental references](designs/README.md): a procedural capital,
four original unit markers and an availability star, each in a separate HTML/JavaScript file.
They guide future Rust builders for objects absent from the frozen prototype. The preview serves
them at `/reference/designs/capital.html`; they do not replace the oracle or its fixtures.
