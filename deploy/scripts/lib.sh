# Shared by the kind scripts. Source it: `. "$(dirname "$0")/lib.sh"`.
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
DEPLOY_DIR="$ROOT_DIR/deploy"
VERSIONS="$DEPLOY_DIR/platform/versions.yaml"

CLUSTER_NAME=${CLUSTER_NAME:-rust-microservices}
NAMESPACE=${NAMESPACE:-shop}
export KIND_HTTP_PORT=${KIND_HTTP_PORT:-8088}
export KIND_KAFKA_PORT=${KIND_KAFKA_PORT:-9093}
export KIND_KAFKA_BROKER_PORT=${KIND_KAFKA_BROKER_PORT:-9094}

# Prints a value of versions.yaml, e.g. `version kind.version`.
version() {
  python3 - "$VERSIONS" "$1" <<'PY'
import sys, yaml
value = yaml.safe_load(open(sys.argv[1]))
for key in sys.argv[2].split("."):
    value = value[key]
print(value)
PY
}

# The pinned kind binary (make kind_tool), falling back to the one on PATH.
KIND="$ROOT_DIR/.tools/bin/kind"
if [[ ! -x "$KIND" ]]; then
  KIND=$(command -v kind || true)
fi

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
