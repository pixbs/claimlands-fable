# ADR 0006: Attribution firewall

Date: 2026-09-09. Status: accepted.

## Context
In the previous iteration, agents kept writing tool names into commit trailers, PR footers and branch
names despite instructions, because higher-priority prompts override repository guidance. The owner
requires that this be technically impossible, not merely discouraged.

## Decision
One pattern list, `.github/policy/banned.txt`, enforced in independent layers: agent configuration
(`.claude/settings.json` disables attribution and a `PreToolUse` guard blocks offending git/gh
commands), git hooks (`commit-msg` strips trailers and rejects banned text, `pre-push` rejects banned
branch names and commits), the required `policy` status check on every PR (branch, title, body,
every commit's message and identities; scrubs known footers from the body), a `create`-event
workflow that deletes banned branches, server rulesets that refuse creation of branches matching the
patterns and require the check on `main`, and squash-only merges so `main` carries only vetted titles.
`cargo xtask lint-repo` applies the same list to tracked files.

## Consequences
A single layer failing (an agent ignoring instructions, a hook not installed) does not let anything
through; the PR simply cannot merge. The cost is a few false positives on words in the list, handled
by rewording. The list is the only place to change when a new tool appears.

## Alternatives
Instructions alone: proven insufficient. Commit-signing requirements: unrelated to content.
Post-merge history rewriting: destructive and too late.
