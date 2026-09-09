# ADR 0005: Trunk-based development, stacked pull requests, squash merges

Date: 2026-09-09. Status: accepted.

## Context
Many small changes from parallel agents must land without long-lived branches, each reviewable on
its own with a live preview, and `main` must stay linear and clean.

## Decision
One trunk (`main`). Branch `<type>/<issue>-<slug>` per issue, one PR per issue, stacks via GitHub's
native stacked pull requests (`gh stack`) when work is large. Squash-only merges with the PR title
(`type(scope): summary`) as the commit message, linear history required, branches deleted on merge.
Every layer of a stack runs full CI and gets a preview.

## Consequences
`main`'s history is a list of reviewed titles. Upper layers of a stack are rebased automatically
after a squash merge by the native stack support (`gh stack sync`). Reviews stay small. Agents need
`gh` and, for stacks, the `gh-stack` extension; the fallback is `git rebase --onto`.

## Alternatives
Merge commits: preserve branch commits but let attribution slip into history and make bisecting
harder. Git-flow or release branches: unnecessary for continuous delivery to previews.
