#!/usr/bin/env bash
# End-to-end check of the order saga across all services.
#
# Requires catalog, customer, order, restaurant and payment to be running (docker compose or
# `cargo run`) plus Kafka. Override the URLs through the environment if needed.
#
#   ./scripts/e2e-saga.sh
#
# Scenario 1: an order within the restaurant stock     -> PENDING -> APPROVED -> PAID
# Scenario 2: an order above the remaining stock         -> PENDING -> CANCELLED (rejected by restaurant)
# Scenario 3: payment fails after the restaurant approved -> APPROVED -> CANCELLED, restaurant stock restored
#
# Scenario 3 stops the payment service and answers the payment request with a FAILED reply through
# `shared/examples/fail_payment.rs` (the payment service never fails an amount the order API
# accepts). It needs cargo on the host and Kafka on KAFKA_HOST. By default it stops/starts the
# docker compose `payment` service; override PAYMENT_STOP_CMD / PAYMENT_START_CMD when payment runs
# elsewhere, or set SKIP_COMPENSATION=1 to skip it.
#
# BASE_URL=http://localhost:8088 sends every call through the api-gateway
# (<BASE_URL>/shop/<service>/...). TARGET=k8s stops/starts payment with `kubectl scale` in
# NAMESPACE (default shop) of the current kube context, e.g. kind:
#
#   BASE_URL=http://localhost:8088 TARGET=k8s KAFKA_HOST=localhost:9093 ./scripts/e2e-saga.sh
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")/.." && pwd)

BASE_URL=${BASE_URL:-}
if [[ -n "$BASE_URL" ]]; then
  CATALOG_URL=${CATALOG_URL:-$BASE_URL/shop/catalog}
  ORDER_URL=${ORDER_URL:-$BASE_URL/shop/order}
  CUSTOMER_URL=${CUSTOMER_URL:-$BASE_URL/shop/customer}
  RESTAURANT_URL=${RESTAURANT_URL:-$BASE_URL/shop/restaurant}
  PAYMENT_URL=${PAYMENT_URL:-$BASE_URL/shop/payment}
fi
CATALOG_URL=${CATALOG_URL:-http://localhost:8080}
ORDER_URL=${ORDER_URL:-http://localhost:8081}
CUSTOMER_URL=${CUSTOMER_URL:-http://localhost:8082}
RESTAURANT_URL=${RESTAURANT_URL:-http://localhost:8083}
PAYMENT_URL=${PAYMENT_URL:-http://localhost:8084}
TIMEOUT=${TIMEOUT:-60}
KAFKA_HOST=${KAFKA_HOST:-localhost:9092}
TARGET=${TARGET:-compose}
NAMESPACE=${NAMESPACE:-shop}
if [[ "$TARGET" == "k8s" ]]; then
  PAYMENT_STOP_CMD=${PAYMENT_STOP_CMD:-k8s_payment stop}
  PAYMENT_START_CMD=${PAYMENT_START_CMD:-k8s_payment start}
else
  COMPOSE="docker compose -f $ROOT_DIR/docker-compose/development/docker-compose.yml"
  PAYMENT_STOP_CMD=${PAYMENT_STOP_CMD:-$COMPOSE stop payment}
  PAYMENT_START_CMD=${PAYMENT_START_CMD:-$COMPOSE start payment}
fi
SKIP_COMPENSATION=${SKIP_COMPENSATION:-0}

command -v jq >/dev/null || { echo "jq is required"; exit 1; }

# Stops or restarts payment in Kubernetes. KEDA would scale a Deployment below its minimum back
# up, so with a ScaledObject the replicas are pinned to 0 (paused) and then handed back to it; a
# plain HPA leaves a Deployment at 0 replicas alone and scales it back to its minimum once restarted.
k8s_payment() {
  local k="kubectl -n $NAMESPACE" keda=""
  $k get scaledobject/payment >/dev/null 2>&1 && keda=1
  if [[ "$1" == "stop" ]]; then
    if [[ -n "$keda" ]]; then
      $k annotate --overwrite scaledobject/payment autoscaling.keda.sh/paused-replicas=0
    else
      $k scale deploy/payment --replicas=0
    fi
    $k wait --for=delete pod -l app.kubernetes.io/name=payment --timeout=120s
  else
    if [[ -n "$keda" ]]; then
      $k annotate scaledobject/payment autoscaling.keda.sh/paused-replicas-
      # KEDA restores the minimum replicas on its next reconcile.
      for _ in $(seq 1 30); do
        [[ $($k get deploy/payment -o jsonpath='{.spec.replicas}') != 0 ]] && break
        sleep 2
      done
    else
      $k scale deploy/payment --replicas=1
    fi
    $k rollout status deploy/payment --timeout=180s
  fi
}

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
fail() { printf '\033[1;31mFAIL: %s\033[0m\n' "$*"; exit 1; }

post() {
  curl --silent --show-error --fail-with-body -X POST "$1" -H 'Content-Type: application/json' -d "$2"
}

# Polls a URL until the jq filter prints the expected value.
wait_for() {
  local url=$1 filter=$2 expected=$3 value=""
  for _ in $(seq 1 "$TIMEOUT"); do
    value=$(curl --silent "$url" | jq -r "$filter" 2>/dev/null || true)
    [[ "$value" == "$expected" ]] && return 0
    sleep 1
  done
  fail "$url: expected $filter == $expected, got '$value'"
}

step "Creating customer"
CUSTOMER_ID=$(post "$CUSTOMER_URL/api/customers" \
  '{"user_name":"e2e-'"$RANDOM"'","first_name":"Saga","last_name":"Tester"}' | jq -r '.data.id')
echo "customer_id=$CUSTOMER_ID"

step "Creating category"
CATEGORY_ID=$(post "$CATALOG_URL/api/categories" \
  '{"title":"E2E '"$RANDOM"'","description":"Saga test category"}' | jq -r '.data.id')

# Creates a product with stock 3 and price 25.00 and waits until the restaurant replicated it.
create_product() {
  local product_id
  product_id=$(post "$CATALOG_URL/api/products" \
    '{"title":"Pizza '"$RANDOM"'","description":"Saga test product","quantity":3,"price":25.0,"category_id":"'"$CATEGORY_ID"'"}' \
    | jq -r '.data.id')
  wait_for "$RESTAURANT_URL/api/products/$product_id" '.data.quantity' 3 >&2
  # The order service replicates customers and products through Kafka too.
  sleep 2
  echo "$product_id"
}

place_order() {
  local product_id=$1 quantity=$2 total
  total=$(jq -n "$quantity * 25")
  post "$ORDER_URL/api/orders" '{
    "customer_id":"'"$CUSTOMER_ID"'",
    "price":'"$total"',
    "items":[{"product_id":"'"$product_id"'","quantity":'"$quantity"',"price":25.0,"sub_total":'"$total"'}]
  }' | jq -r '.data.tracking_id'
}

step "Creating product (stock 3, price 25.00) and waiting for the restaurant to replicate it"
PRODUCT_ID=$(create_product)
echo "product_id=$PRODUCT_ID"

step "Scenario 1: order 2 of 3 in stock -> expect PAID"
TRACKING_ID=$(place_order "$PRODUCT_ID" 2)
echo "tracking_id=$TRACKING_ID"
wait_for "$ORDER_URL/api/orders/$TRACKING_ID" '.data.status' PAID
ORDER_ID=$(curl --silent "$ORDER_URL/api/orders/$TRACKING_ID" | jq -r '.data.id')
echo "order $ORDER_ID is PAID"

wait_for "$RESTAURANT_URL/api/order-approvals/$ORDER_ID" '.data.status' APPROVED
echo "restaurant approval: APPROVED"
wait_for "$RESTAURANT_URL/api/products/$PRODUCT_ID" '.data.quantity' 1
echo "restaurant stock decreased to 1"
wait_for "$PAYMENT_URL/api/payments/orders/$ORDER_ID" '.data.payment_status' COMPLETED
echo "payment: COMPLETED"

step "Scenario 2: order 2 more (only 1 left at the restaurant) -> expect CANCELLED"
TRACKING_ID=$(place_order "$PRODUCT_ID" 2)
echo "tracking_id=$TRACKING_ID"
wait_for "$ORDER_URL/api/orders/$TRACKING_ID" '.data.status' CANCELLED
RESPONSE=$(curl --silent "$ORDER_URL/api/orders/$TRACKING_ID")
echo "failure_messages: $(echo "$RESPONSE" | jq -c '.data.failure_messages')"
ORDER_ID=$(echo "$RESPONSE" | jq -r '.data.id')

wait_for "$RESTAURANT_URL/api/order-approvals/$ORDER_ID" '.data.status' REJECTED
echo "restaurant approval: REJECTED"
STATUS=$(curl --silent -o /dev/null -w '%{http_code}' "$PAYMENT_URL/api/payments/orders/$ORDER_ID")
[[ "$STATUS" == "404" ]] || fail "no payment expected for a rejected order, got HTTP $STATUS"
echo "no payment was made"

if [[ "$SKIP_COMPENSATION" == "1" ]]; then
  echo "Skipping scenario 3 (SKIP_COMPENSATION=1)"
else
  step "Scenario 3: payment fails after approval -> expect CANCELLED and restaurant stock restored"
  PRODUCT_ID=$(create_product)
  echo "product_id=$PRODUCT_ID"
  echo "building the fail_payment helper"
  cargo build --quiet --manifest-path "$ROOT_DIR/shared/Cargo.toml" --example fail_payment \
    --features contracts,kafka,outbox

  echo "stopping the payment service"
  $PAYMENT_STOP_CMD
  trap 'echo "starting the payment service"; $PAYMENT_START_CMD' EXIT

  TRACKING_ID=$(place_order "$PRODUCT_ID" 2)
  echo "tracking_id=$TRACKING_ID"
  wait_for "$ORDER_URL/api/orders/$TRACKING_ID" '.data.status' APPROVED
  ORDER_ID=$(curl --silent "$ORDER_URL/api/orders/$TRACKING_ID" | jq -r '.data.id')
  echo "order $ORDER_ID is APPROVED"
  wait_for "$RESTAURANT_URL/api/products/$PRODUCT_ID" '.data.quantity' 1
  echo "restaurant stock decreased to 1"

  cargo run --quiet --manifest-path "$ROOT_DIR/shared/Cargo.toml" --example fail_payment \
    --features contracts,kafka,outbox -- "$ORDER_ID" "$KAFKA_HOST" "$TIMEOUT"

  wait_for "$ORDER_URL/api/orders/$TRACKING_ID" '.data.status' CANCELLED
  echo "failure_messages: $(curl --silent "$ORDER_URL/api/orders/$TRACKING_ID" | jq -c '.data.failure_messages')"
  wait_for "$RESTAURANT_URL/api/order-approvals/$ORDER_ID" '.data.status' CANCELLED
  echo "restaurant approval: CANCELLED"
  wait_for "$RESTAURANT_URL/api/products/$PRODUCT_ID" '.data.quantity' 3
  echo "restaurant stock restored to 3"
  # When payment restarts it still answers the pending request with COMPLETED; the order service
  # ignores that reply because the saga already FAILED.
fi

printf '\n\033[1;32mAll saga scenarios passed.\033[0m\n'
