#!/usr/bin/env bash
# Installs Ansible (and the Python libraries its modules need) into .tools/ansible, without sudo.
# Current Ansible needs Python >= 3.12, so uv provides a managed interpreter under .tools/python.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TOOLS="$ROOT/.tools"
UV_VERSION=0.13.0
PACKAGES=(ansible==14.5.0 ansible-lint==26.9.0 kubernetes==37.0.1 netaddr==1.3.0)

if [[ ! -x "$TOOLS/uv/bin/uv" ]]; then
  python3 -m venv "$TOOLS/uv"
  "$TOOLS/uv/bin/pip" install --quiet "uv==$UV_VERSION"
fi
export UV_PYTHON_INSTALL_DIR="$TOOLS/python"
uv="$TOOLS/uv/bin/uv"

[[ -x "$TOOLS/ansible/bin/python" ]] || "$uv" venv --quiet --python 3.13 "$TOOLS/ansible"
"$uv" pip install --quiet --python "$TOOLS/ansible/bin/python" "${PACKAGES[@]}"
"$TOOLS/ansible/bin/ansible" --version | head -1
