# Load tests (k6)

Repeatable load against the shop on kind or the homelab, up to 1,000,000 requests, with the
client's view (k6) and the system's reaction (Prometheus) on one Grafana screen.

```bash
make load_tool                         # k6 (version pinned in deploy/platform/versions.yaml) in .tools/bin
make load_seed                         # once per cluster: 50 products with a stock of 1,000,000,000
make load SCENARIO=smoke               # k6 on this machine; TARGET=kind (default) or lab
make load_verify                       # after a run: the shop is consistent
make load_k8s SCENARIO=one-million PARALLELISM=4    # k6-operator runners inside the cluster
```

`make pin_replicas N=3` fixes every service at 3 replicas (KEDA paused) for runs that compare
replica counts; `make unpin_replicas` hands them back to the autoscaler.

## Scenarios

`k6/scenarios/`, each tunable through the environment (`RATE=300 DURATION=10m make load SCENARIO=mixed`):

| Scenario | Load | Defaults |
|---|---|---|
| `smoke` | 1 VU: 3 reads + 1 order, every saga followed to its end | `DURATION=1m` |
| `read-heavy` | GETs only: product pages (catalog), stock (restaurant), product list, order status | `RATE=200` req/s, `DURATION=5m` |
| `place-orders` | POST /api/orders only: the whole saga across order, restaurant and payment | `RATE=50` orders/s, `DURATION=5m` |
| `mixed` | `MIX` reads, the rest orders, as two scenarios so the mix is exact | `RATE=250`, `MIX=0.8`, `DURATION=10m` |
| `stress` | the mix ramps to `MAX_RATE` in `STEPS` steps and stops at the first broken threshold: the knee | `MAX_RATE=3000`, `STEPS=6`, `STEP_DURATION_S=120` |
| `spike` | `BASE_RATE` → `PEAK_RATE` in 10 s, `HOLD`, back down, 2 min to drain | `50` → `1500`, `HOLD=1m` |
| `soak` | the mix at a moderate rate for a long time: leaks, outbox growth, connection churn | `RATE=200`, `DURATION=1h` |
| `one-million` | exactly `TOTAL` requests at `RATE`: `TOTAL × MIX` reads and the rest orders, spread evenly over `TOTAL / RATE` seconds. `PROFILE=max` ramps to `MAX_RATE` instead and stops at the first broken threshold | `TOTAL=1000000`, `RATE=2000` (8m20s), `MIX=0.8` |

Common settings: `BASE_URL` (default `http://localhost:8088` on kind, `https://shop.lab.internal`
on the lab, `http://api-gateway.shop:8070` in the cluster), `DIRECT=1` to call the services at
`/direct/<service>/` without the api-gateway hop, `CUSTOMERS` (20) created per run,
`SAGA_SAMPLE` (0.01) share of orders followed until PAID/CANCELLED, `SAGA_TIMEOUT_S` (60).

Every request is tagged `kind`: `read`, `order`, `saga-poll` (following a sampled saga) or `seed`
(setup). The 1,000,000 of `one-million` are the `read` and `order` requests; the saga polls and the
setup come on top.

## Pass/fail

A run fails (exit code 99) when a threshold breaks (`k6/lib/thresholds.js`):

* `http_req_failed` < 1%
* p95 < 500 ms for reads, < 1 s for order placements
* `dropped_iterations` = 0: k6 kept up with the target rate (`stress` and `PROFILE=max`, which stop at
  the first broken threshold: under 1 dropped iteration per second)
* `saga_timeouts` = 0: every sampled saga finished within `SAGA_TIMEOUT_S`

`make load_verify` (`scripts/verify.sh`) then checks the end state once the sagas drained:
every order is PAID or CANCELLED, no outbox message is FAILED or STARTED, the `.dlq` topics are
empty, and each seeded product's restaurant stock is exactly 1,000,000,000 minus the quantities of
its PAID orders.

## Watching a run

* **Grafana → Shop / Live load test** (5 s refresh): k6's requests/s, latency, VUs, failures,
  dropped iterations and progress toward the request budget, next to the services' requests and
  latency, replicas, CPU and memory per pod, database pool, consumer lag, outbox backlog, saga
  throughput and duration, and node CPU steal. Annotations mark the runs, scaling events and
  Argo CD syncs. k6 sends its metrics with Prometheus remote write as native histograms (`PROM=0`
  to skip; on kind `run.sh` forwards a local port to Prometheus for the run).
* **k6's web dashboard** on http://localhost:5665 while a host run is going.

Each host run leaves `reports/<time>-<scenario>-<target>/` (not in git): `summary.json`, the web
dashboard's `report.html`, the replica counts, the commit, the settings, and `row.md`: the run's
line for the table below, including the largest consumer lag Prometheus saw during the run.
`load_k8s` keeps the runners' logs there instead.

## Honest expectations

Everything shares one 8-core machine: k6, Kafka, 5 Postgres, the services, and on the lab a VM
layer on top (a few % overhead plus CPU steal). The numbers show the **relative** bottlenecks and
how they move with replicas and settings, not production capacity.

Order placement is the expensive path: at least 4 database transactions and 4 Kafka messages per
order across 3 services. The saga backlog (consumer lag and STARTED outbox rows) grows long before
HTTP latency does, which the first kind baselines show: at 80 orders/s HTTP still answered in
< 1 s (p95) while sagas took ~50 s to finish.

## Runs

| Date | Commit | Target | Scenario | Replicas | Rate | Requests | Req/s | p50 | p95 | p99 | Saga p95 | Max lag | Failed | Thresholds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-17 | 78e2b5e+ | kind | smoke | 2 | 1 VU | 143 | 4 | 2 ms | 6 ms | 11 ms | 2.03 s | – | 0.00% | pass |
| 2026-10-17 | 78e2b5e+ | kind | mixed | 2 | 150 | 18,320 | 152 | 4 ms | 15 ms | 43 ms | 3.89 s | – | 0.00% | pass |
| 2026-10-17 | 78e2b5e+ | kind | mixed | 2→3 (KEDA) | 400 | 127,272 | 471 | 13 ms | 310 ms | 662 ms | 53.24 s | 4,888 | 0.00% | FAIL: saga_timeouts (142), dropped_iterations (1,552)¹ |

¹ Part of the dropped iterations came from k6 itself: VUs following a sampled saga were blocked
for up to 60 s and the VU budget didn't account for it (fixed since). The saga timeouts are real:
the order, restaurant and payment consumers fell behind at 80 orders/s. Afterwards the shop
drained and `make load_verify` passed (21,634 orders PAID, nothing lost).

### Lab (2026-10-20)

`one-million` runs `TOTAL=100000 RATE=150 MIX=0.95` on the lab (95% reads, 7.5 orders/s): a
million requests at 2,000/s is beyond one laptop running four VMs (see "Lab vs kind" below).

| Date | Commit | Target | Scenario | Replicas | Rate | Requests | Req/s | p50 | p95 | p99 | Saga p95 | Max lag | Failed | Thresholds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-10-20 | ce2fd1c | lab | smoke | 2 | 1 VU | 273 | 4 | 13 ms | 45 ms | 70 ms | 3.21 s | 26 | 0.00% | pass |
| 2026-10-20 | ce2fd1c | lab | place-orders | 0 payment | 5 | 24,135 | 16 | 29 ms | 626 ms | 1.70 s | – | 7,500 | 0.00% | FAIL (on purpose)² |
| 2026-10-20 | fa71134 | lab | mixed | 3 | 150 | 65,523 | 135 | 150 ms | 4.68 s | 7.96 s | – | 5,981 | 17.62% | FAIL³ |
| 2026-10-20 | 417f554 | lab | mixed | 3 | 150 | 92,153 | 180 | 75 ms | 594 ms | 1.31 s | 44.63 s | 11,646 | 0.00% | FAIL: saga_timeouts, dropped_iterations, order p95⁴ |
| 2026-10-20 | 417f554 | lab | stress (`MAX_RATE=600`) | 2→4 (KEDA) | 100 | 5,625 | 91 | 27 ms | 141 ms | 275 ms | 9.44 s | 253 | 0.00% | stopped: 1 dropped iteration⁵ |
| 2026-10-20 | 1d64708 | lab | one-million (100k) | 3 | 150 | 100,380 | 150 | 18 ms | 77 ms | 169 ms | 3.47 s | 38 | 0.00% | FAIL: dropped_iterations (31)⁶ |
| 2026-10-20 | d86c3d8 | lab | one-million (100k) | 3 | 150 | 100,427 | 150 | 14 ms | 56 ms | 111 ms | 3.73 s | 63 | 0.00% | dropped_iterations (15) only; verify ok |
| 2026-10-20 | d86c3d8 | lab | one-million (100k) | 1 | 150 | 100,655 | 150 | 10 ms | 41 ms | 100 ms | 4.61 s | 50 | 0.00% | dropped_iterations (54) only; verify ok |
| 2026-10-20 | d86c3d8 | lab | mixed, k8s-w2 halted | 3 | 50 | 30,062 | 40 | 23 ms | 6.58 s | 9.53 s | 2.94 s | 31 | 50.89% | FAIL (node loss)⁷ |

² payment paused with KEDA for the lag alert: sagas can't finish.
³ Strimzi rolled the only Kafka broker mid-run (fixed: broker metrics from the JMX exporter).
⁴ `order_items` had no `order_id` index: order's connection pool ran out (fixed). The run also
included a graceful catalog pod delete and an order pod force-kill: 1 failed request of 92,153.
⁵ fewer than 1 dropped iteration per second no longer stops a knee search (fixed).
⁶ `make load_verify` found restaurant stock off by 1-8 units on 49 products: replicas overwrote
each other's stock decrease (fixed in `1.2.0-lab`; the stock was then reset to the expected values).
⁷ k8s-w2 (Kafka, order-db) halted from minute 2 to 8: orders fail until it returns, then every saga
finishes; one reply was dead-lettered during the outage (see the README's known limitations).

The `dropped_iterations` left in the 100k runs are k6's own: on the busy host a VU was sometimes
not free at the scheduled moment (0.02-0.05% of the iterations).

### Lab vs kind

On kind (one machine, no VMs) `mixed` at 150 req/s passed and sagas saturated around 80 orders/s.
The lab holds ~10 orders/s: each consumer pod handles one message at a time, and its handler time
grows from ~95 ms to 400+ ms under the 25-60% CPU steal the VMs see when all four are busy. With
3 vs 1 replica at 150 req/s the HTTP latencies are the same (p95 56 vs 41 ms) and sagas only
slightly faster (3.7 vs 4.6 s p95): more pods on the same 8 cores add no capacity.

## Micro-benchmarks

Where the CPU of a request goes before any I/O (`criterion`, HTML reports in `target/criterion/`):

```bash
cd shared && cargo bench --features contracts --bench avro          # Avro encode/decode of a saga message
cd order && cargo bench -p application --bench create_order         # command -> order + price validation
```

Measured on this machine (2026-10-17): encoding a `RestaurantApprovalRequest` takes ~50 µs and
decoding it ~45 µs, of which **~35 µs is parsing the schema**, which `AvroMessage` does on every
call. Parsing each schema once would cut the Kafka serialization cost by about two thirds.
Mapping an order command to the domain order takes ~4 µs (1 item) to ~24 µs (10 items) and
validating its prices 0.1–0.5 µs: noise next to the database and Kafka round trips.
