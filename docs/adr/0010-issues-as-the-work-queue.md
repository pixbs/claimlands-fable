# ADR 0010: GitHub issues are the work queue

Date: 2026-09-09. Status: accepted.

## Context
The owner hands work to several agent tools with one sentence and wants existing practice, not a
custom tracker. GitHub issues, native dependencies, sub-issues, milestones and assignees already
model readiness, blocking, grouping, ordering and locking.

## Decision
Issue forms with required fields (goal, scope, acceptance criteria, tests, references, blocked-by).
`ready` marks an issue an agent may take; the assignee is the lock; native "blocked by" dependencies
gate availability; milestones order the backlog; sub-issues group ports and rule groups.
`cargo xtask next` lists available issues and `cargo xtask take` claims one and prepares a worktree.
`stale-claims.yml` releases claims older than 24 hours without a PR. Design questions are issues
assigned to the owner that block dependents.

## Consequences
Any agent that can run `gh` can pick work without conversation. The owner steers by labelling
`ready`, ordering milestones and answering questions. Nothing about the queue lives outside GitHub.

## Alternatives
A markdown backlog: drifts and cannot lock. A project-management SaaS: another login and API for
every agent. Custom scripts over a JSON queue: the wheel the owner asked not to reinvent.
