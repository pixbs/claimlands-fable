# Contributing

Everything a contributor needs is in `AGENTS.md` (map, commands, picking work, hard rules) and
`docs/workflow.md` (issues, branches, stacked pull requests, previews, the policy check). The same
rules apply to people and to agents.

Short version:

1. `cargo xtask setup`, then `cargo xtask take` to claim an issue and get a worktree.
2. Implement with tests; keep `cargo xtask check` green.
3. Open a PR titled `type(scope): summary` whose body starts with `Closes #N`; review the preview
   link CI posts; address review; the owner squash-merges.

Design questions go through a *Design question* issue, never through an assumption in code.
