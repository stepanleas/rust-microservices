#!/usr/bin/env bash
# Pins every service of the shop to a fixed number of replicas, e.g. for comparable load-test
# runs, or hands them back to the autoscaler:
#   replicas.sh pin 3      KEDA's paused-replicas annotation on every ScaledObject
#   replicas.sh unpin      removes it; KEDA scales on its triggers again
# TARGET=kind (default) or lab (kubeconfig ~/.kube/lab.yaml). Argo CD leaves the annotation alone:
# it's not part of the chart's manifests.
. "$(dirname "$0")/lib.sh"

case "${TARGET:-kind}" in
  kind) kube=(kubectl --context "kind-$CLUSTER_NAME" -n "$NAMESPACE") ;;
  lab) kube=(kubectl --kubeconfig "$HOME/.kube/lab.yaml" -n "$NAMESPACE") ;;
  *) echo "TARGET must be kind or lab"; exit 1 ;;
esac

annotation=autoscaling.keda.sh/paused-replicas
objects=$("${kube[@]}" get scaledobjects -o name)
[[ -n "$objects" ]] || { echo "no ScaledObjects in namespace $NAMESPACE (is KEDA installed?)"; exit 1; }

case "${1:-}" in
  pin)
    count=${2:?usage: replicas.sh pin <count>}
    step "Pinning every service to $count replicas"
    "${kube[@]}" annotate --overwrite $objects "$annotation=$count"
    # KEDA applies the count on its next reconcile; then wait for the new pods to be ready.
    for object in $objects; do
      deployment=deployment/${object#*/}
      for _ in $(seq 1 30); do
        [[ $("${kube[@]}" get "$deployment" -o jsonpath='{.spec.replicas}') == "$count" ]] && break
        sleep 2
      done
      "${kube[@]}" rollout status "$deployment" --timeout 5m
    done
    ;;
  unpin)
    step "Handing the replicas back to KEDA"
    "${kube[@]}" annotate $objects "$annotation-"
    ;;
  *) echo "usage: replicas.sh pin <count> | unpin"; exit 1 ;;
esac

"${kube[@]}" get deployments
