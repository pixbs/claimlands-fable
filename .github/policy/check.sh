#!/usr/bin/env bash
# Attribution and naming policy, shared by CI (.github/workflows/policy.yml) and the git hooks
# (.githooks/). Every check reads .github/policy/banned.txt.
#
#   check.sh branch <name>
#   check.sh commits <range>          e.g. origin/main..HEAD
#   check.sh commit-msg-file <file>   also used by the commit-msg hook after sanitising
#   check.sh sanitize-msg <file>      drops attribution trailers in place, prints what it removed
#   check.sh pr-title <title>
#   check.sh pr-body-file <file>
#   check.sh text <label> <text>
#
# Exit status 1 on any finding; findings are printed as "policy: <label>: <offending line>".
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
list="$here/banned.txt"
status=0

patterns() {
  # all patterns, scope prefix removed
  grep -vE '^\s*(#|$)' "$list" | sed -E 's/^git +//'
}

scan() {
  # scan <label> <text>: print every line of <text> that matches any pattern
  local label="$1" text="$2" pat
  while IFS= read -r pat; do
    [ -z "$pat" ] && continue
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      if printf '%s\n' "$line" | grep -qiE -- "$pat"; then
        echo "policy: $label: $line   (matches /$pat/)"
        status=1
      fi
    done <<< "$text"
  done < <(patterns)
}

case "${1:-}" in
  branch)
    scan "branch name" "$2"
    case "$2" in
      main|feat/*|fix/*|docs/*|chore/*|task/*|refactor/*|perf/*|test/*|ci/*|build/*|release/*|dependabot/*) ;;
      *) echo "policy: branch name: '$2' must look like <type>/<issue>-<slug> (feat|fix|docs|chore|task|refactor|perf|test|ci|build)"; status=1 ;;
    esac
    ;;
  commits)
    range="$2"
    for sha in $(git rev-list "$range" 2>/dev/null); do
      short="$(git rev-parse --short "$sha")"
      scan "commit $short message" "$(git log -1 --format=%B "$sha")"
      scan "commit $short author" "$(git log -1 --format='%an <%ae>' "$sha")"
      scan "commit $short committer" "$(git log -1 --format='%cn <%ce>' "$sha")"
    done
    ;;
  commit-msg-file)
    scan "commit message" "$(grep -vE '^\s*#' "$2" || true)"
    ;;
  sanitize-msg)
    file="$2"; tmp="$(mktemp)"
    while IFS= read -r line || [ -n "$line" ]; do
      drop=0
      if printf '%s' "$line" | grep -qiE '^(co-authored-by|signed-off-by|generated-by|generated with|🤖)'; then
        while IFS= read -r pat; do
          if printf '%s\n' "$line" | grep -qiE -- "$pat"; then drop=1; break; fi
        done < <(patterns)
      fi
      if [ "$drop" = 1 ]; then echo "policy: removed attribution line: $line"; else printf '%s\n' "$line" >> "$tmp"; fi
    done < "$file"
    mv "$tmp" "$file"
    ;;
  pr-title)
    title="$2"
    scan "PR title" "$title"
    if ! printf '%s' "$title" | grep -qE '^(feat|fix|refactor|perf|test|docs|build|ci|chore)(\([a-z0-9-]+\))?!?: [^ ].{2,}$'; then
      echo "policy: PR title: '$title' must be 'type(scope): summary' with type in feat|fix|refactor|perf|test|docs|build|ci|chore"
      status=1
    fi
    ;;
  pr-body-file)
    body="$(cat "$2")"
    scan "PR body" "$body"
    # SKIP_CLOSES=1 for automated dependency PRs, which have no issue to close.
    if [ "${SKIP_CLOSES:-0}" != 1 ] && ! printf '%s' "$body" | grep -qiE '(closes|fixes|resolves) #[0-9]+'; then
      echo "policy: PR body: must contain 'Closes #<issue>'"
      status=1
    fi
    ;;
  text)
    scan "$2" "$3"
    ;;
  *)
    echo "usage: check.sh branch|commits|commit-msg-file|sanitize-msg|pr-title|pr-body-file|text ..." >&2
    exit 2
    ;;
esac

exit $status
