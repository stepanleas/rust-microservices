#!/usr/bin/env bash
# Encrypts a Secret for the homelab with its Sealed Secrets controller, so it can be committed:
#   make seal NAME=grafana-admin NS=monitoring FROM=~/secrets/grafana.env [OUT=path.yaml]
# FROM is a KEY=value file (kubectl create secret --from-env-file); it never leaves this machine.
# Without OUT the SealedSecret is printed. Only the lab cluster can decrypt it.
. "$(dirname "$0")/lib.sh"

: "${NAME:?NAME=<secret name>}" "${NS:?NS=<namespace>}" "${FROM:?FROM=<KEY=value file>}"
KUBESEAL_VERSION=0.40.0   # matches the controller of the sealed-secrets chart in charts.yaml
KUBESEAL="$ROOT_DIR/.tools/bin/kubeseal"
export KUBECONFIG=${KUBECONFIG_LAB:-$HOME/.kube/lab.yaml}

if ! [[ -x "$KUBESEAL" ]] || ! "$KUBESEAL" --version | grep -q "$KUBESEAL_VERSION"; then
  mkdir -p "$(dirname "$KUBESEAL")"
  url="https://github.com/bitnami/sealed-secrets/releases/download/v$KUBESEAL_VERSION/kubeseal-$KUBESEAL_VERSION-linux-amd64.tar.gz"
  echo "Downloading $url" >&2
  curl -fsSL "$url" | tar -xz -C "$(dirname "$KUBESEAL")" kubeseal
fi

sealed=$(kubectl create secret generic "$NAME" --namespace "$NS" --from-env-file "$FROM" \
  --dry-run=client -o yaml \
  | "$KUBESEAL" --controller-namespace kube-system --controller-name sealed-secrets-controller --format yaml)
if [[ -n "${OUT:-}" ]]; then
  mkdir -p "$(dirname "$OUT")"
  printf '%s\n' "$sealed" > "$OUT"
  echo "Wrote $OUT" >&2
else
  printf '%s\n' "$sealed"
fi
