#!/usr/bin/env bash
# Points this clone at the lab forge as well as GitHub:
#   - remote `lab` = the Forgejo repository (make lab_sync re-pushes the branches to it);
#   - `origin` gets two push URLs, so every `git push` / `git push origin <tag>` goes to both;
#   - git authenticates to the lab with the API token that the forgejo role saved in ~/.lab.
# Idempotent. Undo with: git remote set-url --delete --push origin <lab url>; git remote remove lab
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DOMAIN=$(python3 -c "import yaml; print(yaml.safe_load(open('$ROOT/infra/lab.yml'))['domain'])")
LAB_URL="https://git.$DOMAIN/shop/rust-microservices.git"
TOKEN_FILE="$HOME/.lab/forgejo-token"
cd "$ROOT"

[[ -f "$TOKEN_FILE" ]] || { echo "$TOKEN_FILE is missing: run 'make lab_provision'" >&2; exit 1; }

if git remote get-url lab >/dev/null 2>&1; then
  git remote set-url lab "$LAB_URL"
else
  git remote add lab "$LAB_URL"
fi

# The first explicit push URL replaces the implicit one, so GitHub is added back explicitly.
github_url=$(git remote get-url origin)
# Only the configured ones: before the first, `get-url --push` reports GitHub implicitly.
push_urls=$(git config --get-all remote.origin.pushurl || true)
grep -qxF "$github_url" <<<"$push_urls" || git remote set-url --add --push origin "$github_url"
grep -qxF "$LAB_URL" <<<"$push_urls" || git remote set-url --add --push origin "$LAB_URL"

# Single quotes: the token is read at push time and never stored in .git/config.
git config "credential.https://git.$DOMAIN.helper" \
  '!f() { test "$1" = get && echo username=lab-admin && echo "password=$(cat ~/.lab/forgejo-token)"; }; f'

git remote -v
