#!/usr/bin/env bash
# Creates the kind cluster and installs the platform charts of deploy/platform (in the order of
# charts.yaml), then the kind GatewayClass. Idempotent: re-running upgrades in place.
. "$(dirname "$0")/lib.sh"

[[ -n "$KIND" ]] || { echo "kind not found: run 'make kind_tool'"; exit 1; }
export KIND_NODE_IMAGE=$(version kind.node)

if "$KIND" get clusters | grep -qx "$CLUSTER_NAME"; then
  step "kind cluster $CLUSTER_NAME exists"
else
  step "Creating kind cluster $CLUSTER_NAME ($KIND_NODE_IMAGE)"
  envsubst < "$DEPLOY_DIR/kind/cluster.yaml" | "$KIND" create cluster --config - --wait 120s
fi
kubectl config use-context "kind-$CLUSTER_NAME" >/dev/null

while read -r name repo chart chart_version namespace; do
  step "Installing $name $chart_version"
  values=(-f "$DEPLOY_DIR/platform/$name/values.yaml")
  [[ -f "$DEPLOY_DIR/platform/$name/values-kind.yaml" ]] && values+=(-f "$DEPLOY_DIR/platform/$name/values-kind.yaml")
  if [[ "$repo" == oci://* ]]; then
    source=("$repo")
  else
    source=("$chart" --repo "$repo")
  fi
  helm upgrade --install "$name" "${source[@]}" --version "$chart_version" \
    --namespace "$namespace" --create-namespace "${values[@]}" --wait --timeout 5m \
    | grep -E '^(STATUS|REVISION):'
done < <(python3 - "$CHARTS" <<'PY'
import sys, yaml
for c in yaml.safe_load(open(sys.argv[1])):
    if c.get("kind", True):
        print(c["name"], c["repo"], c.get("chart", "-"), c["version"], c["namespace"])
PY
)

step "Applying the kind GatewayClass"
kubectl apply -f "$DEPLOY_DIR/platform/envoy-gateway/kind/gateway-class.yaml"

step "Cluster ready"
kubectl get nodes
