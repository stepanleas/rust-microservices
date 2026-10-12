#!/usr/bin/env bash
# Creates the Ansible Vault password (~/.lab/vault-pass) and, on first run, the encrypted vault
# with random lab passwords. Never overwrites an existing vault.
set -euo pipefail

ANSIBLE_DIR=$(cd "$(dirname "$0")/../ansible" && pwd)
VAULT="$ANSIBLE_DIR/inventory/group_vars/all/vault.yml"
PASS_FILE="$HOME/.lab/vault-pass"

mkdir -p "$HOME/.lab"
chmod 700 "$HOME/.lab"
if [[ ! -f "$PASS_FILE" ]]; then
  if [[ -f "$VAULT" ]]; then
    echo "$VAULT exists but $PASS_FILE does not: restore the password file from your backup." >&2
    exit 1
  fi
  (umask 077; python3 -c 'import secrets; print(secrets.token_urlsafe(32))' > "$PASS_FILE")
  echo "Created $PASS_FILE (back it up: it decrypts the lab vault)"
fi

if [[ ! -f "$VAULT" ]]; then
  random() { python3 -c "import secrets; print(secrets.token_$1($2))"; }
  # The plaintext only ever exists in a private temp file; the vault appears already encrypted.
  plain=$(mktemp)
  trap 'shred -u "$plain" 2>/dev/null || rm -f "$plain"' EXIT
  cat > "$plain" <<YAML
vault_forgejo_admin_password: "$(random urlsafe 24)"
# Forgejo runners register offline with a shared 40-hex-digit secret.
vault_forgejo_runner_secret: "$(random hex 20)"
vault_registry_ci_password: "$(random urlsafe 24)"
YAML
  ANSIBLE_VAULT_PASSWORD_FILE="$PASS_FILE" ansible-vault encrypt --output "$VAULT" "$plain" </dev/null 2>&1 | cat
  head -1 "$VAULT" 2>/dev/null | grep -q '^\$ANSIBLE_VAULT' || { rm -f "$VAULT"; echo "Encrypting the vault failed" >&2; exit 1; }
  echo "Created the encrypted vault $VAULT"
fi
