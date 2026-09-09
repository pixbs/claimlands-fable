#!/usr/bin/env bash
# PreToolUse guard for the Bash tool: blocks git/gh commands that would write banned attribution
# text into commits, branches or pull requests (see .github/policy/banned.txt). Everything else
# passes through untouched. Exit 2 blocks the tool call and shows the message to the agent.
set -uo pipefail
input="$(cat)"
case "$input" in
  *"git commit"*|*"gh pr create"*|*"gh pr edit"*|*"git checkout -b"*|*"git switch -c"*|*"git branch"*|*"git push"*|*"gh stack"*) ;;
  *) exit 0 ;;
esac
root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
list="$root/.github/policy/banned.txt"
[ -f "$list" ] || exit 0
while IFS= read -r pat; do
  case "$pat" in ''|'#'*) continue ;; esac
  pat="${pat#git }"
  if printf '%s' "$input" | grep -qiE -- "$pat"; then
    echo "blocked: this git/gh command contains text banned by .github/policy/banned.txt (pattern /$pat/). Remove AI names, co-author trailers and 'generated with' footers, then retry." >&2
    exit 2
  fi
done < "$list"
exit 0
