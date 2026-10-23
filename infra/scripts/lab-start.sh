#!/usr/bin/env bash
# Boots the halted lab in stages, so the laptop isn't starting everything at once. The ops VM
# comes first (the lab's DNS and registry), but its CI runner stays stopped until Postgres, Kafka
# and Argo CD are up: booting all four VMs with a CI job compiling left every pod restarting on
# probe timeouts under heavy CPU steal.
set -euo pipefail

cd "$(dirname "$0")/.."
export KUBECONFIG=${KUBECONFIG:-$HOME/.kube/lab.yaml}
WAIT=${WAIT:-20m}
PAUSED=/opt/lab/node-exporter/ci-runner-paused

# Waits for a condition, but keeps booting if it doesn't come: the rest may still be useful.
wait_for() {
  echo "Waiting for $*"
  kubectl wait --timeout="$WAIT" "$@" || echo "WARNING: not ready after $WAIT: $*" >&2
}

./scripts/vagrant.sh up --no-provision --no-parallel ops
./scripts/vagrant.sh ssh ops -c 'sudo docker stop forgejo-runner >/dev/null'
./scripts/vagrant.sh up --no-provision --no-parallel k8s-cp k8s-w1 k8s-w2

echo "Waiting for the API server"
until kubectl get --raw /readyz &>/dev/null; do sleep 5; done
wait_for --for=condition=Ready node --all
wait_for --for=condition=Ready -n shop cluster.postgresql.cnpg.io --all
wait_for --for=condition=Ready -n shop kafka/kafka
wait_for --for=condition=Available -n argocd deployment/argo-cd-argocd-repo-server

# Starting the runner afresh also unsticks it after a halt with jobs in flight. A runner paused
# on purpose (make lab_quiet) stays paused.
./scripts/vagrant.sh ssh ops -c "test -e $PAUSED || sudo docker start forgejo-runner >/dev/null"
kubectl get nodes
