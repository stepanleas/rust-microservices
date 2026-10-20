#!/usr/bin/env bash
# Installs/upgrades the rust-microservices chart with the kind values and waits for it.
. "$(dirname "$0")/lib.sh"

CHART="$DEPLOY_DIR/helm/rust-microservices"
kubectl config use-context "kind-$CLUSTER_NAME" >/dev/null

step "Deploying rust-microservices to namespace $NAMESPACE"
helm dependency update "$CHART" >/dev/null
# Postgres and Kafka come up first; the services crash-loop until their database answers,
# so allow a generous timeout on the first install.
helm upgrade --install rust-microservices "$CHART" -f "$CHART/values-kind.yaml" \
  --namespace "$NAMESPACE" --create-namespace --wait --timeout 15m

step "Waiting for Kafka and the topics"
kubectl -n "$NAMESPACE" wait kafka/kafka --for=condition=Ready --timeout=5m
kubectl -n "$NAMESPACE" wait kafkatopic --all --for=condition=Ready --timeout=2m

step "Waiting for the gateway"
for _ in $(seq 1 60); do
  curl -fsS "http://localhost:$KIND_HTTP_PORT/api/health/ready" >/dev/null 2>&1 && break
  sleep 2
done
curl -fsS "http://localhost:$KIND_HTTP_PORT/api/health/ready"
echo
kubectl -n "$NAMESPACE" get pods
