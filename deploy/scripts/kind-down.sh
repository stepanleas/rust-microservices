#!/usr/bin/env bash
# Deletes the kind cluster (and everything in it).
. "$(dirname "$0")/lib.sh"
"$KIND" delete cluster --name "$CLUSTER_NAME"
