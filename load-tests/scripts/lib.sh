# Shared by the load-test scripts. Source it: `. "$(dirname "$0")/lib.sh"`.
. "$(dirname "${BASH_SOURCE[0]}")/../../deploy/scripts/lib.sh"

LOAD_DIR="$ROOT_DIR/load-tests"
K6="$ROOT_DIR/.tools/bin/k6"
TARGET=${TARGET:-kind}

# kubectl for the target: kind (the default) or the homelab.
case "$TARGET" in
  kind) KUBE=(kubectl --context "kind-$CLUSTER_NAME" -n "$NAMESPACE") ;;
  lab) KUBE=(kubectl --kubeconfig "$HOME/.kube/lab.yaml" -n "$NAMESPACE") ;;
  *) echo "TARGET must be kind or lab"; exit 1 ;;
esac

# psql in the CloudNativePG instance of <service>-db: `psql_in order "select 1"`.
psql_in() {
  "${KUBE[@]}" exec "$1-db-1" -c postgres -- psql -U postgres -d app -tAq -c "$2"
}
