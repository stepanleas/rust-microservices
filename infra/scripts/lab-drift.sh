#!/usr/bin/env bash
# Drift check: runs the whole provisioning in check mode and fails when it would change anything,
# i.e. when the lab no longer matches the code. It runs here and not as a CI job on the ops VM:
# the playbooks need the lab CA, the vault password and every VM's SSH key, all kept in ~/.lab
# and infra/.vagrant on this machine.
# REVISION=<branch>: the branch Argo CD is expected to follow (as make lab_bootstrap).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
log=$(mktemp)
trap 'rm -f "$log"' EXIT

status=0
"$ROOT/infra/scripts/ansible.sh" ansible-playbook site.yml --check --diff \
  ${REVISION:+-e argocd_revision="$REVISION"} | tee "$log" || status=$?

drifted=$(sed -n '/^PLAY RECAP/,$p' "$log" | grep -E 'changed=[1-9]|unreachable=[1-9]|failed=[1-9]' || true)
if [[ $status -ne 0 || -n $drifted ]]; then
  echo >&2
  echo "DRIFT: the lab doesn't match the code (see the diffs above; make lab_provision applies it)" >&2
  [[ -n $drifted ]] && echo "$drifted" >&2
  exit 1
fi
echo "No drift: the lab matches the code."
