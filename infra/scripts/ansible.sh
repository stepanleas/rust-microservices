#!/usr/bin/env bash
# Runs an Ansible command from infra/ansible with the tools of `make lab_tools`, creating the
# vault on first use. Example: infra/scripts/ansible.sh ansible-playbook site.yml
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
VENV="$ROOT/.tools/ansible"
[[ -x "$VENV/bin/ansible-playbook" ]] || { echo "Ansible is not installed: run 'make lab_tools'" >&2; exit 1; }
export PATH="$VENV/bin:$PATH"
export ANSIBLE_CONFIG="$ROOT/infra/ansible/ansible.cfg"

"$ROOT/infra/scripts/lab-secrets.sh"
cd "$ROOT/infra/ansible"
exec "$@"
