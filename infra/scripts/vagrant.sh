#!/usr/bin/env bash
# Runs vagrant from infra/ with the libvirt provider. Right after `make lab_host` the current
# session isn't in the libvirt group yet, so the command goes through `sg libvirt` until re-login.
set -euo pipefail

cd "$(dirname "$0")/.."
export VAGRANT_DEFAULT_PROVIDER=libvirt
command -v vagrant >/dev/null || { echo "Vagrant is not installed: run 'make lab_host'" >&2; exit 1; }

if id -nG | tr ' ' '\n' | grep -qx libvirt; then
  exec vagrant "$@"
fi
exec sg libvirt -c "vagrant $(printf '%q ' "$@")"
