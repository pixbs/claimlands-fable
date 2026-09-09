# Workflow

## Issues are the queue

| Concept | GitHub feature | Meaning |
|---|---|---|
| Work item | issue from a form (`Feature`, `Bug`, `Task`, `Design question`) | acceptance criteria, scope, tests, references |
| Readiness | label `ready` | specified and unblocked; an agent may take it |
| Lock | assignee | whoever is assigned owns it; `stale-claims` releases claims older than 24 h without an open PR |
| Order | milestone, then `prio:p0…p3`, then number | `cargo xtask next` sorts this way and shows only the earliest open milestone |
| Blocking | native issue dependencies ("blocked by") | closing the blocker unblocks dependents automatically |
| Epics | native sub-issues | one parent per prototype section or rule group |
| Area | `area:<crate>` (auto from paths on PRs) | routes attention |
| Decisions | `type:question` + `needs-design`, assigned to the owner | blocks the issues that depend on the answer |

Hand-off to any agent is one line: *Read AGENTS.md, run `cargo xtask take`, implement the issue, open the PR.*

## Claiming and working

```bash
cargo xtask next          # what is available
cargo xtask take 42       # assign yourself, create ../claimlands-wt/42 on feat/42-<slug>
cd ../claimlands-wt/42 && cargo xtask setup
```

Each worktree is an independent checkout; parallel agents never share a working directory.
`Cargo.lock` is the only file two PRs commonly both touch; a PR that adds a dependency carries the
`deps` label and explains the addition.

## Branches, commits, pull requests

- Branch `<type>/<issue>-<slug>`; types `feat fix docs chore task refactor perf test ci build`.
- Commits: plain statements of what changed. No trailers, no co-authors, no tool names.
- PR title `type(scope): summary`, scope = crate or area (`feat(cl-scenery): forest crowns`). The
  title becomes the squash commit on `main`; the body starts with `Closes #N` and follows the template.
- Squash-only merges, linear history, branch deleted on merge, one approving review by the owner.

## Stacks

Large work is a stack of small PRs, each reviewable alone:

```bash
gh extension install github/gh-stack     # once
gh stack init                            # on the first branch
gh stack add feat/42-part-2              # next layer
gh stack submit                          # push and open/update the PRs as a stack
gh stack sync                            # after a layer merges: rebase the rest onto main
```

Every layer runs the full CI and gets its own preview. Merge bottom-up. Without the extension the
fallback is `git rebase --onto main <merged-branch> <next-branch>` after each merge.

## Gates on every PR

| Check | What it enforces |
|---|---|
| `ci` | fmt, clippy (host + wasm32), tests, docs, `cargo deny`, `cargo xtask lint-repo`, coverage floor for `cl-rules`, web bundle |
| `policy` | branch name, PR title format, `Closes #N`, and no banned text in title, body, commits, author or committer identity (`.github/policy/banned.txt`); posts one sticky comment |
| `preview` | deploys `dist/` to Cloudflare Pages and comments the URL (game and `/reference/` prototype) |
| labels | `area:*` from changed paths |

`policy` also scrubs known attribution footers from the PR body before checking. If it still fails,
rename the branch (`git branch -m`), reword commits (`git commit --amend`, `git rebase -i`) and edit
the title or body; the check re-runs on every edit.

Local layers of the same firewall: `.githooks/commit-msg` strips attribution trailers and refuses
banned text, `.githooks/pre-push` refuses banned branch names and commits, and the agent tool
project settings disable attribution and block git commands carrying banned text. Server rulesets
on `main` require PR + `ci` + `policy`, forbid force pushes and non-linear history, and refuse the
creation of branches whose names match the banned patterns.

## After merge

`main` deploys to the production preview URL and, with the nightly run, builds Android, iOS and web
(`mobile.yml`). A failure there opens or updates a `ci-failure` issue.

## Visual changes

The prototype is the oracle. A PR that changes how anything looks carries `visual-change`, updates
the fixtures or baselines it invalidates, and says why in the body. Everything else must leave
`fixtures/` untouched.
