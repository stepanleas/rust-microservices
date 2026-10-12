#!/usr/bin/env bash
# Prints the crates touched between two commits as a JSON list, for the ci.yml matrix:
#   scripts/ci/changed-crates.sh <base> <head>
# A change to `shared` (or to the CI itself) selects every crate, since all of them build against
# the local `shared` directory. Without a usable base (a new branch, a force push that dropped it)
# the base is the merge base with origin/main, or every crate if that is unknown too.
# Plain git instead of an action, so it works the same on GitHub and Forgejo.
set -euo pipefail

CRATES=(shared catalog customer order restaurant payment api-gateway)
ALL_PATTERN='^(shared/|\.github/workflows/ci\.yml$|scripts/ci/changed-crates\.sh$)'

base=${1:-}
head=${2:-HEAD}

all() {
  printf '%s\n' "${CRATES[@]}" | jq -R . | jq -cs .
  exit 0
}

if [[ -z "$base" || "$base" =~ ^0+$ ]] || ! git cat-file -e "$base^{commit}" 2>/dev/null; then
  base=$(git merge-base origin/main "$head" 2>/dev/null) || all
fi

changed=$(git diff --name-only "$base...$head")
grep -qE "$ALL_PATTERN" <<<"$changed" && all

for crate in "${CRATES[@]}"; do
  if grep -q "^$crate/" <<<"$changed"; then echo "$crate"; fi
done | jq -R . | jq -cs .
