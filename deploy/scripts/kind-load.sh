#!/usr/bin/env bash
# Builds the production images (tag `local`) and loads them into the kind nodes: all 6, or only
# SERVICES="order api-gateway". Deployments already running them are restarted, since the tag
# stays `local`.
. "$(dirname "$0")/lib.sh"

read -ra SERVICES <<<"${SERVICES:-catalog customer order restaurant payment api-gateway}"
TAG=${TAG:-local}

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  step "Building images (tag $TAG)"
  make -C "$ROOT_DIR" build_images version="$TAG" services="${SERVICES[*]}"
fi

step "Loading images into kind cluster $CLUSTER_NAME"
images=()
for service in "${SERVICES[@]}"; do
  images+=("artellas/rust-$service:$TAG")
done
"$KIND" load docker-image --name "$CLUSTER_NAME" "${images[@]}"

running=()
for service in "${SERVICES[@]}"; do
  if kubectl --context "kind-$CLUSTER_NAME" -n "$NAMESPACE" get deploy "$service" >/dev/null 2>&1; then
    running+=("deploy/$service")
  fi
done
if ((${#running[@]})); then
  step "Restarting ${running[*]}"
  kubectl --context "kind-$CLUSTER_NAME" -n "$NAMESPACE" rollout restart "${running[@]}"
  for deployment in "${running[@]}"; do
    kubectl --context "kind-$CLUSTER_NAME" -n "$NAMESPACE" rollout status "$deployment" --timeout 5m
  done
fi
