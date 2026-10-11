#!/usr/bin/env bash
# Builds the 6 production images (tag `local`) and loads them into the kind nodes.
. "$(dirname "$0")/lib.sh"

SERVICES=(catalog customer order restaurant payment api-gateway)
TAG=${TAG:-local}

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  step "Building images (tag $TAG)"
  make -C "$ROOT_DIR" build_images version="$TAG"
fi

step "Loading images into kind cluster $CLUSTER_NAME"
images=()
for service in "${SERVICES[@]}"; do
  images+=("artellas/rust-$service:$TAG")
done
"$KIND" load docker-image --name "$CLUSTER_NAME" "${images[@]}"
