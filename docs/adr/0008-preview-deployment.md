# ADR 0008: A playable preview for every pull request

Date: 2026-09-09. Status: accepted.

## Context
The owner reviews behaviour, not only code, and does not want effort spent on review-only pages. The
web build is the development target, so the game itself is the preview.

## Decision
`preview.yml` builds the real web bundle (`cargo xtask web`, which also serves the frozen prototype at
`/reference/`) and deploys it to Cloudflare Pages: a branch URL per PR, the production URL for
`main`. A sticky PR comment carries the links. Secrets live only in the repository's Actions secrets.

## Consequences
Every PR is playable within minutes of CI; A/B against the prototype costs nothing extra. Mobile
builds are not previews; they run on `main` and nightly (`mobile.yml`) and file an issue on failure.
Cloudflare's free tier suffices; the project is `claimlands-fable`.

## Alternatives
GitHub Pages with per-PR folders: needs a `gh-pages` branch, cleanup jobs and repository size care.
Artifacts only: not clickable. A dedicated review page: the token spend the owner ruled out.
