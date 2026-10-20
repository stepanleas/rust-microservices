#!/usr/bin/env bash
# Checks the shop is consistent after a load test, once the sagas drained (up to DRAIN_TIMEOUT_S,
# 600):
#   1. every order is PAID or CANCELLED (none left PENDING / APPROVED / CANCELLING);
#   2. no outbox message is FAILED or still STARTED (order, restaurant, payment);
#   3. the dead letter topics are empty;
#   4. the restaurant's stock of each seeded product = STOCK - the quantities of its PAID orders.
# TARGET=kind (default) | lab.
. "$(dirname "$0")/lib.sh"

DRAIN_TIMEOUT_S=${DRAIN_TIMEOUT_S:-600}
STOCK=${STOCK:-1000000000}
failed=0
pass() { printf '  \033[32mok\033[0m    %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m  %s\n' "$*"; failed=1; }

step "Waiting for the sagas to finish (up to ${DRAIN_TIMEOUT_S}s)"
deadline=$((SECONDS + DRAIN_TIMEOUT_S))
while :; do
  open=$(psql_in order "select count(*) from orders where status in ('pending', 'approved', 'cancelling')")
  [[ "$open" == 0 || $SECONDS -ge $deadline ]] && break
  echo "  $open orders still in progress"
  sleep 10
done

step "1. Orders"
psql_in order "select status, count(*) from orders group by status order by status" | sed 's/^/  /'
if [[ "$open" == 0 ]]; then pass "no order is stuck"; else bad "$open orders still PENDING/APPROVED/CANCELLING"; fi

step "2. Outboxes"
for service in order restaurant payment; do
  counts=$(psql_in "$service" "select
    count(*) filter (where outbox_status = 'FAILED'), count(*) filter (where outbox_status = 'STARTED')
    from outbox_messages")
  IFS='|' read -r failed_rows started_rows <<<"$counts"
  if [[ "$failed_rows" == 0 && "$started_rows" == 0 ]]; then
    pass "$service: no FAILED or STARTED messages"
  else
    bad "$service: $failed_rows FAILED, $started_rows STARTED"
  fi
done

step "3. Dead letter topics"
kafka_pod=$("${KUBE[@]}" get pods -l strimzi.io/broker-role=true -o name | head -1)
dead=$("${KUBE[@]}" exec "${kafka_pod#pod/}" -c kafka -- /opt/kafka/bin/kafka-get-offsets.sh \
  --bootstrap-server localhost:9092 --topic-partitions '.*\.dlq' 2>/dev/null \
  | awk -F: '$3 > 0 {print "  " $1 " partition " $2 ": " $3; n += $3} END {print n + 0}')
total=$(tail -1 <<<"$dead")
if [[ "$total" == 0 ]]; then pass "no message"; else sed '$d' <<<"$dead"; bad "$total dead letters"; fi

step "4. Restaurant stock (STOCK=$STOCK)"
paid=$(psql_in order "select i.product_id, sum(i.quantity) from order_items i
  join orders o on o.id = i.order_id where o.status = 'paid' group by i.product_id")
stock=$(psql_in restaurant "select id, quantity from products where title like 'k6-%'")
[[ -n "$stock" ]] || bad "no seeded products (make load_seed)"
mismatches=$(awk -F'|' -v stock="$STOCK" '
  NR == FNR { paid[$1] = $2; next }
  { expected = stock - paid[$1]; checked++
    if ($2 != expected) { print "  " $1 ": " $2 " in stock, expected " expected; n++ } }
  END { print n + 0 " " checked + 0 }' <(echo "$paid") <(echo "$stock"))
read -r wrong checked <<<"$(tail -1 <<<"$mismatches")"
if [[ "$wrong" == 0 ]]; then
  pass "$checked products: stock = $STOCK - the PAID quantities"
else
  sed '$d' <<<"$mismatches"
  bad "$wrong of $checked products"
fi

echo
if [[ "$failed" == 0 ]]; then
  printf '\033[1;32mThe shop is consistent.\033[0m\n'
else
  printf '\033[1;31mInconsistencies found.\033[0m\n'
  exit 1
fi
