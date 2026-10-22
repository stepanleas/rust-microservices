# Rust Microservices

A food-ordering shop built as five Rust microservices and an API gateway, in one repository.
The services talk to each other through Kafka (Avro messages) and each one owns its PostgreSQL
database. Placing an order runs an **orchestrated saga**: the order service asks the restaurant
to approve the order, then asks the payment service to take the payment, and undoes the
restaurant step when the payment fails. Every saga message leaves its service through a
**transactional outbox**.

Around the code there is a complete local DevOps setup with no cloud dependency: Helm charts
for a kind cluster and for a homelab of KVM virtual machines (Vagrant + Ansible + kubeadm),
CI/CD that runs the same workflows on GitHub and on a local Forgejo, GitOps with Argo CD,
metrics, logs, traces and alerts, autoscaling, and k6 load tests up to 1,000,000 requests.

## Contents

1. [Overview](#1-overview)
   - [Services](#services)
   - [Architecture](#architecture)
   - [Tech stack](#tech-stack)
2. [Business logic](#2-business-logic)
   - [Domain model per service](#domain-model-per-service)
   - [Reference data replication](#reference-data-replication)
   - [The order saga](#the-order-saga)
   - [State machines](#state-machines)
   - [Compensation](#compensation-payment-failed-after-approval)
   - [Idempotency](#idempotency)
   - [Topics and message contracts](#topics-and-message-contracts)
   - [Transactional outbox](#transactional-outbox)
   - [Consuming: retries and dead letter topics](#consuming-retries-and-dead-letter-topics)
   - [Errors and HTTP status codes](#errors-and-http-status-codes)
3. [REST API](#3-rest-api)
4. [Code architecture](#4-code-architecture)
5. [Running it](#5-running-it)
6. [Kubernetes deployment](#6-kubernetes-deployment)
7. [Homelab](#7-homelab)
8. [CI/CD and GitOps](#8-cicd-and-gitops)
9. [Scaling out and load balancing](#9-scaling-out-and-load-balancing)
10. [Observability](#10-observability)
11. [Load testing and benchmarks](#11-load-testing-and-benchmarks)
12. [Configuration](#12-configuration)
13. [Tests](#13-tests)
14. [Project layout](#14-project-layout)
15. [Homelab verification results](#homelab-verification-results)
16. [Known limitations and next steps](#known-limitations-and-next-steps)

---

## 1. Overview

### Services

| Service     | Port | Database (compose host port) | Responsibility |
|-------------|------|------------------------------|----------------|
| catalog     | 8080 | `catalog_db` (5432)          | Categories and products. Publishes `product-*` events. |
| order       | 8081 | `order_db` (5433)            | Places orders, **orchestrates the saga**, order tracking. |
| customer    | 8082 | `customer_db` (5434)         | Customers. Publishes `customer-*` events. |
| restaurant  | 8083 | `restaurant_db` (5435)       | Approves or rejects orders against its own product stock. |
| payment     | 8084 | `payment_db` (5436)          | Takes the payment of approved orders (a local record, no payment provider). |
| api-gateway | 8070 | –                            | Single entry point: `/shop/<service>/**` → `<service>/**`, correlation id, circuit breaker. |

Every service (not the gateway) serves Swagger UI at `http://localhost:<port>/swagger-ui/` and
the OpenAPI document at `http://localhost:<port>/api-docs/openapi.json`.

### Architecture

```mermaid
flowchart LR
    client([Client / k6]) --> envoy[Envoy Gateway<br/>Kubernetes only]
    envoy --> gw[api-gateway<br/>axum]
    client -. local .-> gw

    gw -->|/shop/catalog| catalog
    gw -->|/shop/customer| customer
    gw -->|/shop/order| order
    gw -->|/shop/restaurant| restaurant
    gw -->|/shop/payment| payment

    catalog --- cdb[(catalog_db)]
    customer --- cudb[(customer_db)]
    order --- odb[(order_db<br/>+ outbox)]
    restaurant --- rdb[(restaurant_db<br/>+ outbox)]
    payment --- pdb[(payment_db<br/>+ outbox)]

    catalog -- product-* --> kafka{{Kafka}}
    customer -- customer-* --> kafka
    order <-- saga requests / replies --> kafka
    restaurant <-- approval request / response --> kafka
    payment <-- payment request / response --> kafka
```

The same picture as plain text:

```
                  ┌───────────── HTTP ─────────────┐
 client ──▶ (Envoy) ──▶ api-gateway ──▶ catalog  customer  order  restaurant  payment
                                          │         │        │        │          │
                                       Postgres  Postgres Postgres Postgres  Postgres
                                          │         │        │        │          │
                                          └─────────┴──── Kafka (Avro) ─────────┘
   catalog, customer: publish after commit        order, restaurant, payment: outbox + relay
```

### Tech stack

| Area | Technology |
|---|---|
| Language | Rust, edition 2024 (images build with `lukemathwalker/cargo-chef:latest-rust-1.98.1`) |
| HTTP | actix-web 4 (services), axum 0.8 + reqwest (api-gateway) |
| API docs | utoipa + utoipa-swagger-ui |
| Validation | validator |
| Money | rusty-money (order, catalog), rust_decimal (restaurant, payment) |
| Database | PostgreSQL 18 (compose: `postgres:18-alpine`), Diesel 2.3 with r2d2 pools and embedded migrations |
| Messaging | Apache Kafka 4.3.1 (KRaft), `rdkafka` 0.39 (librdkafka built from source), Avro (`apache-avro` 0.22) |
| Observability | `tracing` + `tracing-subscriber`, `metrics` + `metrics-exporter-prometheus`, OpenTelemetry (OTLP/HTTP) |
| Tests | mockall, testcontainers (Postgres, Kafka), actix test server, criterion benches |
| Containers | Docker, multi-stage builds (cargo-chef → `debian:trixie-slim`), docker compose |
| Kubernetes | kind v0.33.0 (node v1.36.4); homelab: kubeadm 1.37.1, containerd 2.4.1, Cilium 1.20.2 |
| Operators | CloudNativePG (+ PgBouncer), Strimzi, Envoy Gateway, KEDA, k6-operator, Sealed Secrets |
| Observability stack | kube-prometheus-stack (Prometheus, Alertmanager, Grafana), Loki, Tempo, Grafana Alloy |
| Infrastructure as code | Vagrant + vagrant-libvirt (KVM), Ansible |
| Lab services | dnsmasq, Caddy, Zot registry, Forgejo + Actions runner, ntfy, node-exporter |
| CI/CD | GitHub Actions and Forgejo Actions (same workflow files), Argo CD |
| Load tests | k6 v2.3.0 |

Pinned versions live in `deploy/platform/versions.yaml`, `deploy/platform/charts.yaml` and
`infra/ansible/inventory/group_vars/all/main.yml`.

---

## 2. Business logic

### Domain model per service

All ids are typed wrappers around a UUID (`shared::domain::value_objects`: `OrderId`,
`OrderItemId`, `CustomerId`, `ProductId`, `CategoryId`), so ids of different entities can't be
mixed up. The domain crates have no I/O.

#### catalog

| Aggregate | Fields | Rules |
|---|---|---|
| `Category` | id, title, description | Title and description are required (request validation). |
| `Product` | id, title, description, quantity, price (USD `Money`), category | The category must exist. Quantity ≥ 0, price ≥ 1. A category with products can't be deleted (foreign key `ON DELETE RESTRICT`). |

Creating, updating and deleting a product publishes `product-created`, `product-updated` or
`product-deleted` **after** the database commit (no outbox, see
[limitations](#known-limitations-and-next-steps)).

#### customer

| Aggregate | Fields | Rules |
|---|---|---|
| `Customer` | id, user_name, first_name, last_name | All three names are required (request validation). |

Creating and updating a customer publishes `customer-created` / `customer-updated` after the
commit. The service has only `POST` and `PUT` endpoints, no `GET`.

#### order

| Aggregate / entity | Fields | Rules |
|---|---|---|
| `Order` | id, customer_id, tracking_id, price (USD `Money`), items, status, failure_messages | See `Order::validate` below. Status changes only through `approve`, `pay`, `cancel`. |
| `OrderItem` | id, order_id, product_id, quantity, price, sub_total | Valid when `price > 0` and `sub_total == price × quantity`. |
| `OrderSaga` | id, order_id, status, started_at | `restaurant_approved`, `payment_completed`, `fail` (see [State machines](#state-machines)). |
| `Customer` | id, user_name, first_name, last_name | Local copy, replicated from customer. |
| `Product` | id, title, quantity, price | Local copy, replicated from catalog. |

Placing an order (`CreateOrder`) checks, in this order:

1. the customer exists in the local customer table;
2. every product exists in the local product table;
3. each item's quantity is at most the product's **catalog** quantity;
4. `Order::validate`: the order is `PENDING`, its price is not zero, every item is valid
   (`sub_total == price × quantity`), and the order price equals the sum of the items'
   `sub_total`.

A failure is a business-rule error (HTTP 422), for example
`Total price: $50.00 is not equal to order items total: $40.00!`. The request itself is
validated first (HTTP 400): at least one item, quantity ≥ 1, prices ≥ 1. The item prices are
not compared with the catalog prices; the client states them and the order checks that they
add up.

#### restaurant

| Aggregate / service | Fields | Rules |
|---|---|---|
| `Product` | id, title, quantity (= stock), price | `has_stock_for`, `decrease_stock` (no negative stock), `increase_stock`. |
| `OrderApproval` | id, saga_id, order_id, status, failure_messages, created_at | Created `APPROVED` or `REJECTED`. `cancel()` only from `APPROVED`. |
| `RestaurantApprovalDomainService` | – | Decides an approval request (below). |

`RestaurantApprovalDomainService::approve_order` first adds up the requested quantities per
product (an order that lists a product twice counts both lines). Then it collects a failure
message for every problem:

- the order has no products;
- a quantity is ≤ 0;
- a product is unknown: `Product <id> is not available in the restaurant`;
- the stock is too low: `Insufficient stock for product <title>: requested <n>, available <m>`.

Any failure rejects the whole order and leaves the stock untouched. Otherwise the stock of every
product is lowered and the order is approved. The decision is all-or-nothing.

#### payment

| Aggregate | Fields | Rules |
|---|---|---|
| `Payment` | id, order_id, customer_id, price, payment_status, failure_messages | `process()`: a price > 0 is `COMPLETED`, anything else `FAILED` with `Total price must be greater than zero!`. |

Payment statuses are `COMPLETED`, `CANCELLED` and `FAILED`. At most one payment exists per order
(`payments.order_id` is unique). Since the order API only accepts prices ≥ 1, a saga payment
never fails in practice; the end-to-end test injects a failure (see [Tests](#13-tests)).

The REST endpoints `POST`/`PUT /api/payments` manage payments by hand. They publish
`payment-created` / `payment-updated` directly to Kafka (no outbox). No service consumes these
topics.

### Reference data replication

Before an order can be placed, the order and restaurant services need local copies of the data
they validate against. They build them from events:

```
customer ──customer-created / customer-updated──────────────▶ order
catalog  ──product-created / product-updated / product-deleted──▶ order, restaurant
```

- **order** stores customers and products (with the catalog quantity and price). It uses them
  to check the customer, the product ids and the catalog quantity.
- **restaurant** stores products, and their `quantity` is its **stock**. Every approved order
  lowers it, so the restaurant can reject an order even though the catalog still lists enough.
  A later `product-updated` event overwrites the stock with the catalog quantity.

Saving is an upsert and deleting a missing product is a no-op, so redelivered events are
harmless.

### The order saga

```mermaid
sequenceDiagram
    autonumber
    actor Client
    participant O as Order service
    participant K as Kafka
    participant R as Restaurant service
    participant P as Payment service

    Client->>O: POST /api/orders
    Note over O: one DB transaction:<br/>order PENDING + saga STARTED<br/>+ outbox RESTAURANT_APPROVAL_REQUEST
    O-->>Client: 201 { tracking_id }
    O->>K: outbox relay → restaurant-approval-request
    K->>R: restaurant-approval-request
    Note over R: one DB transaction:<br/>approval APPROVED/REJECTED + stock update<br/>+ outbox RESTAURANT_APPROVAL_RESPONSE
    R->>K: outbox relay → restaurant-approval-response
    K->>O: restaurant-approval-response

    alt approved
        Note over O: one DB transaction:<br/>order APPROVED + saga PROCESSING<br/>+ outbox PAYMENT_REQUEST
        O->>K: outbox relay → payment-request
        K->>P: payment-request
        Note over P: one DB transaction:<br/>payment row + outbox PAYMENT_RESPONSE
        P->>K: outbox relay → payment-response
        K->>O: payment-response
        alt payment COMPLETED
            Note over O: order PAID, saga SUCCEEDED
        else payment FAILED or CANCELLED
            Note over O: one DB transaction:<br/>order CANCELLED + saga FAILED<br/>+ outbox RESTAURANT_APPROVAL_CANCEL_REQUEST
            O->>K: outbox relay → restaurant-approval-cancel-request
            K->>R: restaurant-approval-cancel-request
            Note over R: one DB transaction:<br/>stock given back + approval CANCELLED
        end
    else rejected
        Note over O: order CANCELLED (+ failure messages), saga FAILED
    end

    Client->>O: GET /api/orders/{tracking_id}
    O-->>Client: { status: PAID | CANCELLED | ... }
```

Step by step:

1. **Place the order.** `POST /api/orders` runs the checks above. In one database transaction
   the order service stores the order as `PENDING`, creates an `order_sagas` row with status
   `STARTED`, and writes a `RESTAURANT_APPROVAL_REQUEST` row into `outbox_messages`. The client
   gets a `tracking_id` right away (HTTP 201).
2. **Ask the restaurant.** The order service's outbox relay publishes the row to
   `restaurant-approval-request` and marks it `COMPLETED`.
3. **Restaurant decides.** The restaurant runs `RestaurantApprovalDomainService`. In one
   transaction it stores an `order_approvals` row (`APPROVED`, or `REJECTED` with the failure
   messages), lowers the stock when approved, and writes a `RESTAURANT_APPROVAL_RESPONSE` outbox
   row. Its relay publishes the reply to `restaurant-approval-response`.
4. **Order reacts** (`RestaurantApprovalSaga`):
   - **approved:** order `PENDING → APPROVED`, saga `STARTED → PROCESSING`, and a
     `PAYMENT_REQUEST` outbox row, all in one transaction. The relay publishes it to
     `payment-request`.
   - **rejected:** order `PENDING → CANCELLED` with the restaurant's failure messages, saga
     `STARTED → FAILED`. Nothing was reserved, so there is nothing to compensate. The saga ends.
5. **Payment.** The payment service inserts a `payments` row (`COMPLETED` for a positive
   amount, otherwise `FAILED`) and a `PAYMENT_RESPONSE` outbox row in one transaction. Its relay
   publishes the reply to `payment-response`.
6. **Order completes** (`PaymentSaga`):
   - **completed:** order `APPROVED → PAID`, saga `PROCESSING → SUCCEEDED`.
   - **failed or cancelled:** order `APPROVED → CANCELLED` with the payment failure messages,
     saga `PROCESSING → FAILED`, and a `RESTAURANT_APPROVAL_CANCEL_REQUEST` outbox row in the
     same transaction (see [Compensation](#compensation-payment-failed-after-approval)).

Follow an order with `GET /api/orders/{tracking_id}`.

### State machines

Order status (order service, `orders.status`; the API returns it upper-case):

```
PENDING ──restaurant approved──▶ APPROVED ──payment completed──▶ PAID
   │                                 │
   └──restaurant rejected──▶ CANCELLED ◀──payment failed──┘
```

The status enum and the table also allow `CANCELLING`, but no code path sets it today.

Saga status (order service, `order_sagas.status`):

```
STARTED ──restaurant approved──▶ PROCESSING ──payment completed──▶ SUCCEEDED
   │                                  │
   └──restaurant rejected──▶ FAILED ◀─┴──payment failed
```

| Saga status  | Meaning                                         | Order status |
|--------------|-------------------------------------------------|--------------|
| `STARTED`    | Waiting for the restaurant                      | `PENDING`    |
| `PROCESSING` | Restaurant approved, waiting for the payment    | `APPROVED`   |
| `SUCCEEDED`  | Payment completed                               | `PAID`       |
| `FAILED`     | Restaurant rejected the order or payment failed | `CANCELLED`  |

Approval status (restaurant service, `order_approvals.status`):

```
APPROVED ──cancel request──▶ CANCELLED
REJECTED   (final)
```

The domain aggregates reject illegal transitions, such as paying an order the restaurant has not
approved: `Order::approve/pay/cancel`, `OrderSaga::restaurant_approved/payment_completed/fail`
and `OrderApproval::cancel` return an error instead.

### Compensation (payment failed after approval)

When the restaurant approved an order, it already lowered its stock. If the payment then fails,
the order service undoes that step:

1. In the transaction that cancels the order and fails the saga, the order service writes a
   `RESTAURANT_APPROVAL_CANCEL_REQUEST` outbox row with the order's products and quantities.
   This only happens when the failure arrives while the saga is `PROCESSING`, i.e. after the
   restaurant approved. A rejected order has nothing to give back.
2. The relay publishes it to `restaurant-approval-cancel-request`, keyed by the order id.
   `saga_compensations_total` counts it.
3. The restaurant (`CancelOrderApproval`) looks up the approval of that order. If it is
   `APPROVED`, it adds the quantities back to the stock and marks the approval `CANCELLED`, in
   one transaction. The update only matches a row that is still `APPROVED`, so a concurrent
   duplicate can't give the stock back twice. A missing, `REJECTED` or already `CANCELLED`
   approval is logged and ignored. Products that no longer exist are skipped with a warning.
   No reply is sent.

`GET /api/order-approvals/{order_id}` then shows `CANCELLED`.

### Idempotency

Delivery is at-least-once (outbox relay and Kafka consumer), so every consumer is idempotent:

| Consumer | Mechanism |
|---|---|
| order, saga replies | A reply is applied only while the saga is in the expected status (`STARTED` for restaurant replies, `PROCESSING` for payment replies). Unknown sagas and replies for a saga that moved on are logged and ignored. The status update itself is a compare-and-set (`UPDATE … WHERE status = expected`), so two concurrent deliveries can't both win. |
| order, reference data | Upserts; deleting a missing product is a no-op. |
| restaurant, approval request | At most one approval per order (`order_approvals.order_id` is unique). A request for an order it already answered returns the stored approval and writes nothing. |
| restaurant, cancel request | Only an `APPROVED` approval is cancelled (see above). |
| restaurant, reference data | Upserts; deleting a missing product is a no-op. |
| payment, payment request | At most one payment per order (`payments.order_id` is unique). A duplicate request is skipped. |

A duplicate saga reply is a successful no-op, not an error, so it never ends up in a dead letter
topic.

### Topics and message contracts

All messages are Avro object containers with the schema embedded. On the saga topics the key is
the order id, so all messages of one order land on the same partition and keep their order.

The schemas and the Rust models live in one place, the `shared` crate (`contracts` feature), so
producer and consumer can't drift apart. Topic names are constants in
`shared::contracts::topics`.

| Topic | Producer → consumer | Schema (`shared/avro/schemas/`) |
|---|---|---|
| `restaurant-approval-request` | order → restaurant | `restaurant_approval_request_avro_model.avsc` |
| `restaurant-approval-response` | restaurant → order | `restaurant_approval_response_avro_model.avsc` |
| `restaurant-approval-cancel-request` | order → restaurant | `restaurant_approval_cancel_request_avro_model.avsc` |
| `payment-request` | order → payment | `payment_request_avro_model.avsc` |
| `payment-response` | payment → order | `payment_response_avro_model.avsc` |
| `customer-created`, `customer-updated` | customer → order | `customer_*_event_avro_model.avsc` |
| `product-created`, `product-updated`, `product-deleted` | catalog → order, restaurant | `product_*_event_avro_model.avsc` |
| `payment-created`, `payment-updated` | payment → (nobody) | `payment_*_event_avro_model.avsc` |

Every saga message carries `id`, `saga_id` and `order_id`. Prices are decimal strings (`"60.00"`),
statuses are upper-case strings (`APPROVED`/`REJECTED`, `COMPLETED`/`FAILED`/`CANCELLED`), and
`failure_messages` is an array of strings.

All topics have **12 partitions**. In docker compose the `kafka-init` container creates them
from `docker-compose/*/kafka/topics/*.json`; in Kubernetes they are Strimzi `KafkaTopic`
resources. The consumers also allow topic auto-creation, so a fresh broker works too.

### Transactional outbox

A service must never change its database and then publish to Kafka as two independent steps. If
it crashes in between, or Kafka is down, the other services never hear about the change. The
order, restaurant and payment services therefore use the **transactional outbox** pattern
(`shared::outbox`):

1. The business change and the message are written in **the same database transaction**. The
   message goes to the `outbox_messages` table:

   ```sql
   CREATE TABLE outbox_messages (
       id uuid PRIMARY KEY,
       saga_id uuid NOT NULL,
       message_type VARCHAR(64) NOT NULL,     -- e.g. RESTAURANT_APPROVAL_REQUEST, PAYMENT_REQUEST
       payload JSONB NOT NULL,                -- message body, mapped to Avro when published
       outbox_status VARCHAR(16) NOT NULL,    -- STARTED | COMPLETED | FAILED
       attempts INT NOT NULL DEFAULT 0,
       created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
       processed_at TIMESTAMP,
       trace_context VARCHAR(128)             -- W3C traceparent of the writing transaction
   );
   CREATE INDEX idx_outbox_messages_status_created_at ON outbox_messages (outbox_status, created_at);
   ```

2. An **outbox relay** thread (`OutboxRelay`) polls every `OUTBOX_POLL_INTERVAL_MS`. In one
   transaction it claims up to `OUTBOX_BATCH_SIZE` `STARTED` rows, oldest first, with
   `SELECT … FOR UPDATE SKIP LOCKED`. It publishes each one, marks it `COMPLETED` and commits.
   Every replica runs a relay. Rows claimed by one relay are skipped by the others, so scaling
   out never sends a message twice.
3. If publishing fails, the row stays `STARTED` and `attempts` goes up, so the next poll retries
   it. After `OUTBOX_MAX_ATTEMPTS` failures the row becomes `FAILED` and an error is logged.
4. Every 5 minutes the relay deletes `COMPLETED` rows processed more than
   `OUTBOX_RETENTION_HOURS` ago, 5,000 rows per statement, so the table stays small under load.
5. Every 15 seconds the relay publishes the backlog gauges (`outbox_messages{status}`,
   `outbox_oldest_started_age_seconds`).

The message types per service:

| Service | Outbox message types |
|---|---|
| order | `RESTAURANT_APPROVAL_REQUEST`, `PAYMENT_REQUEST`, `RESTAURANT_APPROVAL_CANCEL_REQUEST` |
| restaurant | `RESTAURANT_APPROVAL_RESPONSE` |
| payment | `PAYMENT_RESPONSE` |

The Kafka producer (`shared::kafka::KafkaProducer`) is idempotent with `acks=all`, `lz4`
compression and `linger.ms=5`. It connects lazily, and `send` waits for the broker's
acknowledgement, so a row is marked `COMPLETED` only once Kafka has the message. A send that
can't be delivered within 10 s fails, and the relay retries the row on its next poll.

### Consuming: retries and dead letter topics

Every consuming service (order, restaurant, payment) runs one listener thread
(`shared::kafka::listen`) with one `MessageHandler` that dispatches by topic. The listener is a
member of the service's consumer group (`order-service-group`, `restaurant-service-group`,
`payment-service-group`), so the replicas of a service split the partitions between them
(cooperative-sticky rebalancing) instead of each reading every message. For each message:

1. The handler is called. Handlers return errors instead of logging and swallowing them. An
   unknown topic or an undecodable message is an error too.
2. A failing message is retried with exponential backoff (`RetryPolicy::default()`: 5 attempts,
   500 ms doubling up to 10 s), so a short database outage does not lose a saga reply. Messages
   of one partition are handled in order, so a retried message holds back the ones behind it.
3. After the last attempt the message is published unchanged (same key) to `<topic>.dlq`, for
   example `payment-response.dlq`, and the listener moves on.
4. The offset is stored only once the message was handled or dead-lettered, and only stored
   offsets are committed (every second and on rebalance). A crash causes redelivery, never
   loss.

The Kafka client is librdkafka (`rdkafka`, built from source), so building needs a C toolchain,
`make` and `perl`.

### Errors and HTTP status codes

Use cases return `shared::error::AppError`. The presentation layer maps it to a status code and
an `ErrorResponse` body:

| `AppError` | HTTP | Example |
|---|---|---|
| `NotFound` | 404 | No order with this tracking id |
| `Validation` | 400 | Malformed JSON, failed request validation |
| `BusinessRule` | 422 | Unknown customer, insufficient catalog quantity, prices that don't add up |
| `Conflict` | 409 | A payment for this order already exists; an illegal saga transition |
| `Internal` | 500 | Database or broker unreachable; details are logged, not returned |

---

## 3. REST API

| Service | Endpoint | Description |
|---|---|---|
| catalog | `GET`, `POST /api/categories`; `GET`, `PUT`, `DELETE /api/categories/{id}` | Category CRUD |
| catalog | `GET`, `POST /api/products`; `GET`, `PUT`, `DELETE /api/products/{id}` | Product CRUD, publishes `product-*` |
| customer | `POST /api/customers`, `PUT /api/customers/{id}` | Create / update a customer (no `GET`), publishes `customer-*` |
| order | `POST /api/orders` | Place an order, starts the saga (201, `{ data: { tracking_id } }`) |
| order | `GET /api/orders/{tracking_id}` | Order status, price, items and failure messages |
| restaurant | `GET /api/order-approvals/{order_id}` | The restaurant's decision: `APPROVED` / `REJECTED` / `CANCELLED` |
| restaurant | `GET /api/products`, `GET /api/products/{id}` | Restaurant stock (replicated from catalog) |
| payment | `GET /api/payments/{id}` | A payment |
| payment | `GET /api/payments/orders/{order_id}` | The payment of an order |
| payment | `POST /api/payments`, `PUT /api/payments/{id}` | Manual payment management, publishes `payment-*` |
| all services | `GET /api/health/{startup,live,ready}` | Kubernetes probes |
| all services | `GET /api/info` | `{ environment }` |
| all services + gateway | `GET /metrics` | Prometheus metrics |

Successful responses are wrapped in `{ "data": … }`. Request and response schemas, examples and
error codes are in each service's Swagger UI.

### Gateway routing

The api-gateway (`api-gateway/`, axum) forwards `/shop/<service>/<rest>` to
`<upstream>/<rest>` with the prefix stripped, keeping the query string:

```bash
curl localhost:8070/shop/order/api/orders/$TRACKING      # -> http://localhost:8081/api/orders/$TRACKING
```

- Upstreams come from `MICROSERVICE_<NAME>_URL` (default `http://localhost:<port>`). An unknown
  service name answers 404.
- It sets or forwards `X-Correlation-ID`, adds `X-Forwarded-For`, `-Proto`, `-Prefix` and
  `-Host`, drops hop-by-hop headers and returns `X-Response-Time`.
- It continues or starts the W3C trace and passes `traceparent` upstream.
- Each upstream has a count-based **circuit breaker** (resilience4j semantics: sliding window,
  failure-rate threshold, half-open trial calls). Only transport errors and timeouts count as
  failures; a 5xx is a valid answer. While open, the gateway answers 503 at once.
- The gateway serves `GET /api/health/live`, `GET /api/health/ready` and `GET /metrics` (no
  startup endpoint).

In Kubernetes, Envoy Gateway sends all traffic to the api-gateway. On kind and the lab,
`/direct/<service>/...` also reaches each service without the api-gateway, to measure that
hop.

### Trying it by hand

Against the docker compose stack (`make up`):

```bash
# 1. a customer and a product (stock 3)
CUSTOMER=$(curl -s -XPOST localhost:8082/api/customers -H 'Content-Type: application/json' \
  -d '{"user_name":"jdoe","first_name":"John","last_name":"Doe"}' | jq -r .data.id)
CATEGORY=$(curl -s -XPOST localhost:8080/api/categories -H 'Content-Type: application/json' \
  -d '{"title":"Pizza","description":"Pizzas"}' | jq -r .data.id)
PRODUCT=$(curl -s -XPOST localhost:8080/api/products -H 'Content-Type: application/json' \
  -d '{"title":"Margherita","description":"Classic","quantity":3,"price":25.0,"category_id":"'$CATEGORY'"}' | jq -r .data.id)

# 2. place an order (price = sum of sub_totals, sub_total = price * quantity)
TRACKING=$(curl -s -XPOST localhost:8081/api/orders -H 'Content-Type: application/json' -d '{
  "customer_id":"'$CUSTOMER'","price":50.0,
  "items":[{"product_id":"'$PRODUCT'","quantity":2,"price":25.0,"sub_total":50.0}]}' | jq -r .data.tracking_id)

# 3. follow it: PENDING -> APPROVED -> PAID (usually within a few seconds)
curl -s localhost:8081/api/orders/$TRACKING | jq .data
```

Ordering 2 more right afterwards passes the order service's check (the catalog still lists 3)
but the restaurant rejects it, since only 1 is left in its stock. That order ends `CANCELLED`
with `Insufficient stock for product Margherita: requested 2, available 1`.

---

## 4. Code architecture

### Hexagonal layers per service

Each service is its own Cargo workspace with six crates, one per layer. Dependencies point
inwards: the domain knows nothing about the others.

```
<service>/src/
├── domain/          aggregates/<aggregate>/ (entity, builder, tests), events, value objects,
│                    domain services and errors. No I/O, no framework, no shared feature.
├── application/
│   ├── ports/in/    driving ports: the message-listener traits the messaging adapter calls
│   ├── ports/out/   driven ports: repositories, publishers (outbox ports come from shared)
│   ├── use_cases/   one synchronous struct per use case: `execute(..) -> AppResult<T>`
│   ├── saga/        (order) the saga steps RestaurantApprovalSaga and PaymentSaga
│   └── outbox.rs    (order, restaurant, payment) the service's outbox message types
├── infrastructure/  postgres/<aggregate>.rs (Diesel repository + rows), schema.rs, migrations/
├── messaging/       one Kafka MessageHandler (dispatch by topic), Avro mappers, publishers
├── presentation/    actix-web controllers + OpenAPI docs, request/response DTOs, AppState
└── starter/         config (Settings), wiring: DB pool, use cases, HTTP server,
                     Kafka listener thread, outbox relay thread; main.rs
```

| Layer | Depends on | Talks to |
|---|---|---|
| domain | `shared` (no features) | nothing |
| application | domain, `shared` (`outbox`) | ports only |
| infrastructure | application, `shared` (`db`, `outbox-postgres`) | Postgres |
| messaging | application, `shared` (`contracts`, `kafka`) | Kafka |
| presentation | application, `shared` (`web`) | HTTP clients |
| starter | all of the above, `shared` (`observability`) | – |

HTTP controllers only map DTOs and run a use case on the blocking thread pool
(`shared::web::blocking`), so Diesel and the Kafka producer never block the actix workers.
Migrations are embedded in the binary and run at startup.

### The `shared` crate

`shared` (version 2.0.0) is split into features, so each layer pulls in only what it needs:

| Feature | Contents | Used by |
|---|---|---|
| (none) | `AppError`/`AppResult`, typed ids, `HealthCheck`, `Shutdown`/`Workers`, `trace_context` | domain, application |
| `contracts` | Avro schemas and models of every message, `topics` | messaging |
| `outbox` | generic `OutboxMessage`, outbox ports, `OutboxRelay` | application, messaging |
| `outbox-postgres` | `PostgresOutboxRepository`, `insert_outbox_message` | infrastructure |
| `db` | `connect()`: r2d2 pool, startup retry (120 s), embedded migrations under an advisory lock, `DbHealthCheck`, pool metrics | infrastructure, starter |
| `kafka` | `KafkaProducer`, `KafkaConsumer`, `listen()` with retries and DLQ, `ensure_topics` | messaging, starter |
| `web` | `ApiError`, `ErrorResponse`, `ValidatedJson`, correlation id middleware, `blocking()`, health endpoints, `serve()` | presentation, starter |
| `logging` | `tracing-subscriber` setup (`RUST_LOG`, `LOG_FORMAT=json`) | starter |
| `observability` | `logging` + Prometheus `/metrics` + OTLP trace export; with `web` also the `HttpMetrics` middleware | starter, api-gateway |
| `mocks` | mockall mocks of the outbox ports | tests |

The services depend on it by path (`shared = { path = "../shared" }`). A change to `shared` is
built and tested against every service in the same commit; there is no separate version to tag.
Because of that, the Docker images are built with the repository root as context:

```bash
docker build -f order/Dockerfile -t artellas/rust-order:1.4.0 .
make build_images version=local      # all 6 images in parallel
```

The production `Dockerfile`s use cargo-chef for dependency caching and copy the binary to
`/usr/local/bin/app` in a `debian:trixie-slim` image. The `Dockerfile-dev`s run `cargo watch`
for hot reload.

### api-gateway

A single axum crate (`api-gateway/src/`): `proxy.rs` (forwarding, upstream metrics),
`circuit_breaker.rs`, `correlation_id.rs`, `http_metrics.rs`, `config.rs`. It uses `shared` only
for the `observability` feature.

---

## 5. Running it

| Target | What runs where | Entry point | Needs |
|---|---|---|---|
| docker compose (dev) | 6 services with hot reload, 5 Postgres, Kafka | `make up` | Docker |
| Host + compose infra | services via `cargo run`, Postgres and Kafka in Docker | `make infra_up` | Docker, Rust, C toolchain |
| docker compose (scaled) | production images, 3× order and restaurant | `make scale_up` | Docker, `make build_images version=local` |
| kind | the whole system on a 3-node local cluster with the platform | `make kind_*` | Docker (~6 CPUs, 12–16 GB) |
| Homelab | 4 KVM VMs, kubeadm, GitOps | `make lab_*` | KVM, ~19 GB free RAM |

### Everything in Docker (development, hot reload)

```bash
make up                     # docker compose up -d --build: 5 services + api-gateway, DBs, Kafka
make e2e                    # the end-to-end saga scenarios against it
make down
```

The development containers compile with `cargo watch`, so the first start takes a few minutes.
Each service waits for its database and for `kafka-init` to create the topics. Kafka is a single
KRaft node (`apache/kafka:4.3.1`) with two listeners: `kafka:29092` for containers and
`localhost:9092` for the host.

### Services on the host, infrastructure in Docker

```bash
make infra_up               # 5 Postgres containers + Kafka + topic creation
cd order && cp .env.example .env && cargo run -p starter    # repeat for each service
cd api-gateway && cargo run
```

Each service reads its `.env` (copy `.env.example`); it points to `localhost:<db port>` and
`localhost:9092`.

### Scaled production stack

```bash
make build_images version=local
make scale_up               # production compose + 3 replicas of order and restaurant (replicas=N)
make e2e_scaled             # the saga check through the gateway on :8070
make scale_down
```

### Kubernetes on kind

```bash
make kind_tool      # the pinned kind binary in .tools/bin (deploy/platform/versions.yaml)
make kind_up        # 1 control plane + 2 workers, platform charts and observability stack
make kind_images    # builds the 6 images (tag local) and loads them into the nodes
make kind_deploy    # helm upgrade --install rust-microservices into namespace shop
make kind_e2e       # the saga test through the gateway
make kind_down
```

`kind_images` takes `SERVICES="order api-gateway"` to rebuild only some images, and
`SKIP_BUILD=1` to only load them. Already deployed pods are restarted.

| URL | What |
|---|---|
| http://localhost:8088/shop/<service>/... | the shop through Envoy and the api-gateway (`KIND_HTTP_PORT`) |
| http://localhost:8088/direct/<service>/... | a service without the api-gateway |
| http://localhost:8088/grafana | Grafana (admin/admin, anonymous viewers allowed) |
| `localhost:9093` | Kafka's external listener (`KIND_KAFKA_PORT`), for tools on the host |

For example: `curl localhost:8088/shop/catalog/api/categories`.

### Kubernetes on the homelab

```bash
make lab_tools lab_host     # once: Ansible in .tools/, then libvirt/Vagrant on this machine (sudo)
make lab_up lab_verify      # VMs + provisioning, then the health checks
```

Stop kind first: both together don't fit in 31 GB. See [Homelab](#7-homelab) and
[infra/README.md](infra/README.md).

---

## 6. Kubernetes deployment

```
deploy/
├── kind/cluster.yaml            nodes, node image and host port mappings
├── platform/                    operators and stacks shared by kind and the homelab
│   ├── versions.yaml            pinned kind version, node image and k6 version
│   ├── charts.yaml              pinned platform charts, install order, which target gets which
│   ├── <component>/             values.yaml (+ values-<target>.yaml), kind/ or lab/ manifests
│   └── monitoring/              local chart: alert rules, Grafana dashboards, extra scrape targets
├── argocd/                      the homelab's Argo CD Applications (app of apps)
├── helm/
│   ├── rust-service/            one service: Deployment, Service, ConfigMap, KEDA ScaledObject
│   │                            (or HPA), PDB, probes, ServiceMonitor
│   └── rust-microservices/      umbrella: rust-service aliased per service, CNPG Cluster + Pooler
│                                per service, Strimzi Kafka + 22 KafkaTopics, Gateway + HTTPRoute,
│                                PodMonitors; values-kind.yaml / values-lab.yaml
└── scripts/                     kind-tool / kind-up / kind-load / kind-deploy / kind-down,
                                 replicas (pin/unpin), seal
```

### `rust-service` (library chart)

One chart for every service, used six times through aliases:

- **Deployment**: rolling update with `maxUnavailable: 0`, `maxSurge: 1`; probes on
  `/api/health/startup`, `/live`, `/ready`; `topologySpreadConstraints` across nodes; a 5 s
  `preStop` sleep and `terminationGracePeriodSeconds: 60`; a config checksum annotation, so a
  changed ConfigMap rolls the pods.
- **ConfigMap** with the plain environment; `DATABASE_URL` is assembled from the CloudNativePG
  secret `<cluster>-app` and points at the pooler `<cluster>-pooler`. `DB_MAX_CONNECTIONS` is 10.
- **Service** (ClusterIP), **PodDisruptionBudget** (`minAvailable: 1`), **ServiceMonitor**
  (when the CRD exists).
- **Autoscaling**: a KEDA `ScaledObject` when KEDA is installed, otherwise a CPU
  `HorizontalPodAutoscaler` (70%). See [Scaling out](#9-scaling-out-and-load-balancing).

### `rust-microservices` (umbrella chart)

| Part | Details |
|---|---|
| Services | catalog, customer, order, restaurant, payment, api-gateway; images `<registry>/<namespace>/rust-<service>:<tag>` |
| Postgres | One CloudNativePG `Cluster` per service (`<service>-db`, 1 instance) plus a PgBouncer `Pooler` in **session** mode (prepared statements and the migration advisory lock need it), `maxClientConnections: 200`, `defaultPoolSize: 40` |
| Kafka | Strimzi `Kafka` 4.3.1 with one dual-role (controller + broker) `KafkaNodePool` node, Kafka metrics and the kafka-exporter for consumer lag; 12 topics + 10 `.dlq` topics as `KafkaTopic`, 12 partitions each; optional external listener on NodePorts 30093/30094 |
| Gateway | Gateway API `Gateway` (class `envoy`) + `HTTPRoute` to the api-gateway; optional `/direct/<service>` routes; can attach to an existing Gateway (`parentRef`) |
| Monitoring | PodMonitors for CNPG, Kafka and the kafka-exporter |
| Tracing | `OTEL_EXPORTER_OTLP_ENDPOINT=http://alloy.monitoring:4318`, sampler `parentbased_traceidratio` with `global.tracing.sampleRatio` (1.0; 0.1 on the lab) |

Replicas starting together apply the migrations one at a time: `shared::db::connect` holds a
Postgres advisory lock around them.

### Values per target

| Setting | `values.yaml` (defaults) | `values-kind.yaml` | `values-lab.yaml` |
|---|---|---|---|
| Images | `artellas/rust-<svc>:latest` (Docker Hub) | tag `local`, `pullPolicy: Never` | `registry.lab.internal/shop/rust-<svc>`, tags from `images.yaml` in `lab-env` |
| Replicas (min–max) | 2–6 | 2–3 | 2–4 |
| Kafka storage | 10 Gi | 2 Gi, external listener on the ingress-ready node | 5 Gi |
| Postgres storage | 5 Gi | 1 Gi | 2 Gi |
| Gateway | own Gateway, any host | own Gateway, `directRoutes: true` | `shop.lab.internal` on the shared HTTPS Gateway `lab`, `directRoutes: true` |
| `ENVIRONMENT` | `production` | `kind` | `lab` |

### Platform charts

`deploy/platform/charts.yaml` is the single list of platform charts. `kind-up.sh` installs them
in order on kind; on the lab, Argo CD creates one Application per chart, ordered by sync waves.

| Chart | Version | Namespace | kind | lab | Purpose |
|---|---|---|---|---|---|
| local-path-provisioner | v0.0.37 | local-path-storage | – | ✓ | Default StorageClass on the VMs (kind has its own) |
| metrics-server | 3.14.0 | kube-system | ✓ | ✓ | Resource metrics for CPU autoscaling |
| cloudnative-pg | 0.29.1 | cnpg-system | ✓ | ✓ | Postgres operator |
| strimzi-kafka-operator | 1.2.0 | strimzi | ✓ | ✓ | Kafka operator (KRaft) |
| envoy-gateway | v1.9.2 | envoy-gateway-system | ✓ | ✓ | Gateway API implementation |
| sealed-secrets | 2.20.0 | kube-system | – | ✓ | Encrypted secrets in git |
| kube-prometheus-stack | 92.10.0 | monitoring | ✓ | ✓ | Prometheus, Alertmanager, Grafana, node-exporter, kube-state-metrics |
| loki | 18.16.2 | monitoring | ✓ | ✓ | Logs |
| tempo | 3.2.0 | monitoring | ✓ | ✓ | Traces, span metrics and service graph |
| alloy | 1.13.1 | monitoring | ✓ | ✓ | Node agent: pod logs → Loki, OTLP traces → Tempo |
| keda | 2.21.0 | keda | ✓ | ✓ | Autoscaling on consumer lag and requests/s |
| k6-operator | 4.6.0 | k6-operator | ✓ | ✓ | In-cluster load-test runners |
| monitoring (local) | – | monitoring | ✓ | ✓ | Shop rules, dashboards, ops VM scrape targets |
| argo-cd | 10.10.3 | argocd | – | ✓ | GitOps |

---

## 7. Homelab

A prod-like environment on one machine, with no cloud: four KVM VMs on a private libvirt
network. Full details, first run, day-2 operations and troubleshooting are in
[infra/README.md](infra/README.md); this is the summary.

### Topology

| VM | IP | vCPU / RAM / disk | Runs |
|---|---|---|---|
| `ops` | 192.168.56.10 | 4 / 3.5 GB / 80 GB | dnsmasq, Caddy (TLS), Zot registry + docker.io cache, Forgejo + Actions runner, ntfy, node-exporter, Alloy (ops logs → Loki) |
| `k8s-cp` | 192.168.56.11 | 2 / 4 GB / 30 GB | kubeadm control plane (tainted; only the k6 runners tolerate it) |
| `k8s-w1`, `k8s-w2` | 192.168.56.12, .13 | 4 / 5.5 GB / 60 GB | workloads |
| LoadBalancer pool | 192.168.56.200–.220 | – | Cilium LB-IPAM, announced over ARP; `.200` is the cluster ingress (Envoy) |

That's 18.5 GB of RAM and 14 vCPUs (the VMs return their free memory to the host); CPU is overcommitted on 8 host threads. The topology lives in
one file, `infra/lab.yml`, read by both the `Vagrantfile` and the Ansible inventory
(`inventory/lab.py`).

### Who does what

- **Vagrant** creates empty VMs (CPU, RAM, disk, IP) from `lab.yml`, nothing more.
- **Ansible** turns them, and this machine, into the lab. Its tasks are idempotent: a second
  `make lab_provision` reports `changed=0`. `site.yml` runs common → ops → kubernetes → argocd.
- **Argo CD** manages everything inside Kubernetes once the cluster exists.

| Area | Ansible's job |
|---|---|
| This machine (`make lab_host`, sudo) | libvirt, Vagrant + vagrant-libvirt, the `lab` network (bridge `virbr-lab`), split DNS for `lab.internal` via systemd-resolved, lab CA in the system store and browsers |
| Every VM | apt baseline, chrony, swap off, kernel modules and sysctls, lab CA trusted, `*.lab.internal` resolved by the ops VM |
| ops | dnsmasq, Docker, Caddy, Zot, Forgejo (admin, org `shop`, repo `rust-microservices`, Actions, repo variables), runner, ntfy, node-exporter, Alloy |
| Kubernetes nodes | containerd + runc, pulls through Zot; pinned kubelet/kubeadm/kubectl; `kubeadm init` from a versioned config (no kube-proxy); workers joined; Cilium (kube-proxy replacement, L2 announcements, LB-IPAM, Hubble); kubelet-csr-approver |
| Bootstrap (`argocd` role) | Argo CD, the root Application, the wildcard TLS secret for the Gateway and the lab CA for Argo CD and Prometheus/Alertmanager |
| Day 2 | rolling Kubernetes upgrades, certificate renewal, cluster reset, backups |

The lab's own secrets (Forgejo admin password, runner secret, registry CI password) are random,
created on first use and encrypted with Ansible Vault in
`infra/ansible/inventory/group_vars/all/vault.yml`. The vault password and the lab CA are in
`~/.lab/`: back that directory up.

### DNS, TLS and registry

- **DNS:** dnsmasq on `ops` serves `lab.internal`. Every `*.lab.internal` name resolves to the
  ingress `192.168.56.200`, except the ops names (`ops`, `git`, `registry`, `mirror`, `ntfy`) and
  the VM names. The host routes `~lab.internal` to the ops VM through systemd-resolved.
- **TLS:** Ansible generates a lab root CA in `~/.lab/ca`, name-constrained to `lab.internal`
  and `192.168.56.0/24`. Caddy on `ops` and the cluster Gateway both serve a one-year wildcard
  certificate signed by it; `lab_provision` renews it 30 days before expiry. There is no
  cert-manager: the bootstrap writes the certificate into the `lab-wildcard-tls` secret.
- **Registry:** Zot at `registry.lab.internal` holds the shop images (`shop/rust-<service>`)
  and the CI build cache; `mirror.lab.internal` is an on-demand cache of docker.io. containerd on
  every node pulls through it and falls back to Docker Hub when it is down.
- **Secrets in git:** the Sealed Secrets controller runs in the lab cluster.
  `make seal NAME=… NS=… FROM=<KEY=value file> [OUT=…]` encrypts a Secret that only the lab
  cluster can decrypt, so it is safe to commit. Grafana's admin login is one: its source is
  `~/.lab/grafana-admin.env` and the SealedSecret sits in the lab values of
  kube-prometheus-stack (`extraManifests`).

### Lab URLs

| URL | What |
|---|---|
| https://shop.lab.internal | the shop through the api-gateway (`/shop/<service>/...`, `/direct/<service>/...`) |
| https://git.lab.internal | Forgejo (user `lab-admin`, password in the vault) |
| https://registry.lab.internal | Zot (anonymous pulls, pushes as `forgejo-ci`) |
| https://argocd.lab.internal | Argo CD (`admin`, password in `argocd-initial-admin-secret`) |
| https://grafana.lab.internal | Grafana |
| https://prometheus.lab.internal, https://alertmanager.lab.internal | Prometheus, Alertmanager |
| https://ntfy.lab.internal | ntfy, alerts on the topic `lab-alerts` |

`kubectl --context lab …` uses `~/.kube/lab.yaml`, merged into `~/.kube/config`.

### Day-2 operations

| Target | |
|---|---|
| `make lab_status` | VM states and nodes |
| `make lab_provision` | re-apply the configuration |
| `make lab_verify` | nodes Ready, Cilium healthy, HTTPS with a trusted certificate, pulls through Zot, a LoadBalancer IP reachable from the host; `IDEMPOTENCY=1` also re-runs the provisioning |
| `make lab_ssh VM=k8s-w1` | shell on a VM |
| `make lab_quiet` / `lab_unquiet` | stop/start the CI runner, e.g. during load tests |
| `make lab_snapshot NAME=x` / `lab_restore NAME=x` | VM snapshots |
| `make lab_k8s_upgrade VERSION=1.37.1` | rolling upgrade: control plane, then each worker (drain → upgrade → uncordon) |
| `make lab_renew_certs` | renew kubeadm's certificates |
| `make lab_k8s_reset` | rebuild the cluster on the existing VMs (workloads are lost) |
| `make lab_etcd_backup` / `lab_backup` | etcd snapshot + PKI (and a Forgejo dump) to `~/.lab/backups/<time>/` |
| `make lab_bootstrap [REVISION=<branch>]` | Argo CD + root Application; `REVISION` makes the lab follow a branch |
| `make lab_drift [REVISION=<branch>]` | drift check: provisioning in check mode must change nothing |
| `make lab_remote` / `lab_sync` | push to GitHub and the lab at once / catch the lab up with GitHub's branches |
| `make ci_exec WORKFLOW=infra.yml [JOB=…]` | run a workflow locally with the lab's runner (`forgejo-runner exec`) |
| `make lab_start` | boot the halted lab in stages: ops, then the cluster; CI starts once Postgres, Kafka and Argo CD are Ready |
| `make lab_halt` / `lab_destroy` | stop / delete the VMs (`~/.lab` is kept) |

Only the inventory knows about Vagrant. Moving the lab to 2–3 mini PCs means a static inventory
with the same groups (see [infra/README.md](infra/README.md#moving-to-real-hardware)).

---

## 8. CI/CD and GitOps

### Workflows

The three workflows in `.github/workflows/` run unchanged on GitHub and on the lab's Forgejo.
There is no `.forgejo/workflows`, so Forgejo uses them too.

| Workflow | Trigger | Jobs |
|---|---|---|
| `ci.yml` | pushes to `main`, `feature/*`, `fix/*`; PRs to `main` (Markdown-only changes skipped) | `changes`: `scripts/ci/changed-crates.sh` picks the changed crates; a change to `shared/` or the CI checks all of them. `check` (matrix per crate): `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace --all-features` (testcontainers on the runner's Docker) |
| `release.yml` | tags `<service>-v<semver>`, or manual run with a list of existing tags | `tags`: validates and turns them into a matrix. `release`: builds `<service>/Dockerfile` from the repo root and pushes `<version>`, `<major>.<minor>`, `<major>` (and `latest`). `deploy-lab` (lab only): bumps the tags in `lab-env` |
| `infra.yml` | changes under `infra/`, `deploy/`, `load-tests/` | `ansible-lint`; `promtool check rules`; `helm lint` of every chart and values file; `k6 inspect` of every scenario; `helm template` + `kubeconform` (with and without KEDA) against the lab's Kubernetes version and the CRD catalog |

`changed-crates.sh` uses plain `git diff`, so it behaves the same on both forges. It falls back to
the merge base with `origin/main` when the base commit is unknown.

### One set of workflows, two platforms

Everything that differs comes from repository variables and secrets. The Ansible `forgejo` role
sets them on the lab; on GitHub they are unset and the defaults apply.

| | GitHub | Lab (Forgejo) |
|---|---|---|
| Variables | none | `CI_PLATFORM=lab`, `REGISTRY_HOST=registry.lab.internal`, `IMAGE_NAMESPACE=shop`, `LAB_ENV_REPO`, `BUILDKITD_CONFIG` |
| Image | `artellas/rust-<service>` on Docker Hub | `registry.lab.internal/shop/rust-<service>` |
| Build cache | `type=gha` | `type=registry` in Zot (`cache/rust-<service>`) |
| Secrets | `DOCKER_USERNAME` / `DOCKER_PASSWORD` (environment `Rust Microservices`) | the Zot CI user, `LAB_DEPLOY_TOKEN` |
| `deploy-lab` | skipped | runs |

Release a service:

```bash
git tag order-v1.4.0
git push origin order-v1.4.0      # GitHub: Docker Hub
git push lab order-v1.4.0         # lab: Zot, then a rolling update on shop.lab.internal
```

An existing tag can be released again from **Actions → release → Run workflow** with one or
more tags, e.g. `order-v1.0.0 payment-v1.0.0`; each tag builds in its own job.
`infra/scripts/lab-remote.sh` can add the lab as a second push URL of `origin`, so one
`git push` reaches both.

### GitOps on the lab

```
git push lab order-v1.4.0
  └─▶ Forgejo Actions: release.yml
        ├─ release:    build, push registry.lab.internal/shop/rust-order:1.4.0
        └─ deploy-lab: commit `order: {image: {tag: "1.4.0"}}` to images.yaml in shop/lab-env
              └─▶ Argo CD (polls every 60 s) syncs rust-microservices
                    └─▶ rolling update of deployment/order (maxUnavailable 0)
```

The bot user `lab-deploy` can only write to `lab-env`, so nothing ever commits to `main`, and
`main` stays identical on GitHub and the lab. The tags live in a repository of their own
because an Argo CD application can't combine two revisions of the same repository.

Argo CD runs the **app of apps** in `deploy/argocd`. The root Application, created by Ansible,
renders this chart:

| Application | Source | Sync wave |
|---|---|---|
| one per platform chart (`templates/platform.yaml`) | upstream chart + `deploy/platform/<name>/values*.yaml` | -3 (storage), -2 (operators, observability), -1 (`monitoring`, Argo CD itself) |
| `lab-config` | `deploy/platform/*/lab/*.yaml`, e.g. the HTTPS Gateway on `.200` | 0 |
| `rust-microservices` | `deploy/helm/rust-microservices` + `values-lab.yaml` + `$env/images.yaml` from `lab-env` | 1 |

Every Application syncs automatically with `prune` and `selfHeal`, server-side apply, and
retries while the CRDs of an operator are not there yet.

---

## 9. Scaling out and load balancing

Every service can run several replicas. The pieces that make that safe:

| Concern | How it is handled |
|---|---|
| Readiness | `GET /api/health/ready` takes a pool connection and runs `SELECT 1` within 1 s; it answers 503 while the database is unreachable, so the replica gets no traffic. `live` and `startup` never check dependencies. At startup the service retries the database for up to 120 s instead of crash-looping. |
| Kafka consumers | The replicas of a service form one consumer group and split the 12 partitions of each topic, so up to 12 replicas share the work. A rebalance may redeliver a few messages; the handlers are idempotent. |
| Outbox relays | One per replica, coordinated by `FOR UPDATE SKIP LOCKED`. |
| Migrations | Serialized by a Postgres advisory lock. |
| Graceful shutdown | On SIGTERM the HTTP server stops accepting connections and finishes in-flight requests (up to 30 s). Then the Kafka listener finishes the message in hand and commits its offsets, and the relay finishes the message it is publishing and commits its batch (up to 15 s). A message waiting for a retry is left for redelivery instead of being dead-lettered. The chart adds a 5 s `preStop` sleep and a 60 s grace period. |
| Rolling updates | `maxUnavailable: 0`, `maxSurge: 1`, PodDisruptionBudget `minAvailable: 1`, at least 2 replicas, spread across nodes. |

### Database connections

r2d2 opens the `DB_MAX_CONNECTIONS` connections of a replica at startup and keeps them. Every
request, the Kafka listener and the relay borrow from the same pool, and the relay holds one
connection for the duration of a batch. Without a pooler, keep

```
replicas × DB_MAX_CONNECTIONS  <  max_connections − 3     (Postgres default: 100)
```

For example, 3 replicas × 16 = 48 fits one default Postgres, while 6 × 16 = 96 does not. In
Kubernetes the services use `DB_MAX_CONNECTIONS=10` and connect through PgBouncer (session mode,
up to 200 client connections, a server pool of 40), which bounds the server-side connections.

### Autoscaling (KEDA)

| Service | Triggers | Replicas (kind / lab) |
|---|---|---|
| catalog, customer | CPU 70% | 2–3 / 2–4 |
| order, restaurant, payment | CPU 70% + Kafka consumer lag of their group: one replica per 100 waiting messages, never more than partitions | 2–3 / 2–4 |
| api-gateway | CPU 70% + requests/s from Prometheus (`http_requests_total` without health checks): one replica per 500 req/s | 2–3 / 2–4 |

Without KEDA the chart falls back to a CPU HorizontalPodAutoscaler. For comparable load-test
runs, fix the replicas and pause KEDA:

```bash
make pin_replicas N=3 [TARGET=lab]     # KEDA's paused-replicas annotation on every ScaledObject
make unpin_replicas [TARGET=lab]
```

### Load balancing

- **North-south:** Envoy Gateway balances each request over the api-gateway pods. On the lab its
  Service gets `192.168.56.200` from Cilium LB-IPAM (2 Envoy replicas); on kind it is a NodePort
  mapped to `localhost:8088`.
- **East-west:** the api-gateway reaches the services through ClusterIP Services, which balance
  per **connection**. Keep-alive connections would pin traffic to the pods that existed when they
  opened, so the gateway closes idle upstream connections after 30 s
  (`POOL_IDLE_TIMEOUT_MS`) and keeps at most 32 per host (`POOL_MAX_IDLE_PER_HOST`). New
  connections then spread over the current pods.
- **Kafka:** balanced by the consumer group's partition assignment.
- **Measuring it:** the *Load balancing* dashboard shows the per-pod request skew, partitions
  per consumer pod and the autoscaler inputs. `/direct/<service>/...` bypasses the api-gateway
  to measure its hop.

---

## 10. Observability

### Logs

The services and the gateway log through `tracing`. With `LOG_FORMAT=json` (set in Kubernetes)
every line is a JSON object with the fields of the enclosing spans:

- `correlation_id` for HTTP requests (from `X-Correlation-ID`, generated when missing);
- `topic`, `partition`, `offset` and `key` for Kafka messages (the key is the order id on the
  saga topics);
- `message_id`, `saga_id` and `message_type` for outbox publishes;
- `trace_id` when traces are exported.

Alloy ships the pod logs to Loki. On the lab, Alloy on the ops VM also ships journald and the
Docker containers' logs (Forgejo, Zot, Caddy, the runner) to the same Loki.

### Metrics

Every service and the gateway serve `GET /metrics` in the Prometheus text format:

| Metric | Labels | From |
|---|---|---|
| `http_requests_total`, `http_request_duration_seconds` | `method`, `route` (template), `status` | all |
| `db_pool_connections` | `state` = idle/in_use | services |
| `db_pool_wait_seconds`, `db_pool_timeouts_total` | – | services |
| `kafka_messages_handled_total`, `kafka_handler_duration_seconds` | `topic`, `result` = ok/retry/dlq | order, restaurant, payment |
| `kafka_consumer_assigned_partitions` | `topic` | order, restaurant, payment |
| `outbox_messages` (STARTED/FAILED), `outbox_oldest_started_age_seconds` | `status` | order, restaurant, payment |
| `outbox_publish_total`, `outbox_publish_duration_seconds` | `type`, `result` | order, restaurant, payment |
| `saga_started_total`, `saga_completed_total`, `saga_duration_seconds`, `saga_compensations_total` | `outcome` = paid/rejected/payment_failed | order |
| `restaurant_order_approvals_total`, `restaurant_order_cancellations_total` | `decision` | restaurant |
| `payments_processed_total` | `status` | payment |
| `gateway_upstream_requests_total`, `gateway_upstream_duration_seconds` | `service`, `outcome` = ok/error/timeout | api-gateway |
| `gateway_circuit_breaker_state` (0 closed, 1 half-open, 2 open), `gateway_circuit_breaker_rejected_total` | `breaker` / `service` | api-gateway |

The outbox gauges describe the whole table, so every replica reports the same value. Prometheus
also scrapes CloudNativePG, PgBouncer, Kafka, the kafka-exporter (consumer lag), Envoy, KEDA,
Argo CD and, on the lab, the ops VM (node-exporter, Zot, Forgejo, ntfy).

### Traces

With `OTEL_EXPORTER_OTLP_ENDPOINT` set, spans go out over OTLP/HTTP to the node's Alloy, which
forwards them to Tempo. One trace follows an order all the way:

1. the gateway continues or starts the trace (W3C `traceparent`) and passes it upstream;
2. the service continues it in its HTTP span;
3. the outbox row stores the `traceparent` of the transaction that wrote it (`trace_context`);
4. the relay publishes the message in that trace, with a `traceparent` Kafka header;
5. the consumer continues the trace from the header, so the restaurant's and payment's work and
   their replies land in the same trace.

`OTEL_TRACES_SAMPLER=parentbased_traceidratio` with `OTEL_TRACES_SAMPLER_ARG` keeps a share of the
new traces. Tempo's metrics generator writes span metrics and the service graph to Prometheus.
Grafana links the three signals: a log line's `trace_id` opens the trace, a span opens the logs
of every service it touched, and exemplars lead from latency panels to traces.

### Dashboards

Folder *Shop* in Grafana (`deploy/platform/monitoring/dashboards/shop/`):

| Dashboard | Shows |
|---|---|
| System overview | gateway requests/s, 5xx and p95; error rate per service and top failing routes; saga throughput, outcomes and duration; the slowest gateway requests with a link to their trace |
| Service detail | per service: requests by route, errors, p95, database pool (connections, wait, timeouts), CPU, memory, restarts, and its logs |
| Load balancing | request skew per service, requests/s and CPU per pod, desired vs. current replicas, partitions and messages per consumer pod, gateway upstream latency, Envoy per upstream pod, KEDA trigger inputs |
| Saga & Kafka | consumer lag per group and topic, messages handled/s, handler p95, dead letters, outbox backlog and oldest message, saga duration, outcomes and compensations, restaurant decisions, payments by status |
| Postgres | transactions/s, connections, lock waits, longest transaction, cache hit ratio, size, deadlocks, PgBouncer client and server connections |
| Live load test | k6's load (progress, requests/s by kind, VUs, failures, dropped iterations, client latency, sampled saga duration) next to the shop's reaction (requests/s and p95 per service, replicas, CPU and memory per pod, pool, consumer lag, outbox backlog, sagas, node CPU busy and steal) |

Folder *Infrastructure* (lab only, `dashboards/infrastructure/`):

| Dashboard | Shows |
|---|---|
| Lab / Infrastructure | nodes ready, ops VM, CI runner, Argo CD, firing alerts and certificate expiry at a glance; CPU busy and steal, memory, disk and network per VM; pods per node; Zot registry and Docker Hub mirror storage, requests and pulls served vs. fetched upstream; Argo CD applications and syncs |

### Alerts

Rules in `deploy/platform/monitoring/rules/` (plain Prometheus rule files, checked by
`promtool` in CI, wrapped into `PrometheusRule`s by the `monitoring` chart):

| Alert | Fires when | Severity |
|---|---|---|
| `ShopHighErrorRate` | > 1% 5xx for a service, 5 min | critical |
| `ShopLatencySloBreached` | p95 > 0.5 s, 10 min | warning |
| `ShopCircuitBreakerOpen` | a gateway breaker is open, 1 min | critical |
| `ShopConsumerLagGrowing` | lag > 10 and still growing, 10 min | warning |
| `ShopOutboxStuck` | oldest `STARTED` outbox message > 60 s, 2 min | warning |
| `ShopOutboxMessagesFailed` | any `FAILED` outbox message | warning |
| `ShopDeadLetters` | messages written to a `.dlq` topic in the last 15 min | warning |
| `ShopPodRestarting` | > 2 restarts in 15 min | warning |
| `ShopDbPoolExhausted` | pool wait p95 > 1 s or pool timeouts, 2 min | warning |
| `ShopPostgresDown`, `ShopKafkaDown` | Postgres or Kafka not ready, 2 min | critical |
| `ShopAutoscalerTriggerFailing` | a KEDA scaler errors, 5 min | warning |
| `ShopAutoscalerAtMaximum` | an autoscaler wants more than its maximum, 15 min | warning |
| `LabNodeNotReady` | a node is not Ready, 2 min (lab) | critical |
| `LabDiskAlmostFull` | a root disk is almost full (lab) | warning |
| `LabOpsVmDown`, `LabOpsServiceDown` | the ops VM or one of its services doesn't answer (lab) | critical / warning |
| `LabCertificateExpiring` | a lab certificate expires in < 14 days (lab) | warning |
| `LabCiRunnerOffline` | the Forgejo Actions runner container is not running, 15 min (lab; a `make lab_quiet` pause is ignored for 3 h) | warning |
| `LabArgoAppOutOfSync`, `LabArgoAppDegraded` | an Argo CD application is OutOfSync or Degraded/Missing, 10 min (lab) | warning |

On the lab Alertmanager sends every alert (except `Watchdog` and `InfoInhibitor`) to **ntfy** on
the ops VM: `https://ntfy.lab.internal/lab-alerts`, formatted by ntfy's Alertmanager template.
Subscribe to that topic in a browser or the ntfy app for push notifications.

---

## 11. Load testing and benchmarks

k6 scenarios from a smoke test to exactly 1,000,000 requests, run from this machine or inside
the cluster (k6-operator), with a consistency check afterwards and the results live on Grafana's
*Live load test* dashboard. Full guide, settings and recorded runs:
[load-tests/README.md](load-tests/README.md).

```bash
make load_tool load_seed                              # once: k6, then 50 products with a huge stock
make load SCENARIO=mixed                              # TARGET=kind (default) | lab
make load_verify                                      # the shop is consistent afterwards
make load_k8s SCENARIO=one-million PARALLELISM=4      # runners inside the cluster
```

| Scenario | Load |
|---|---|
| `smoke` | 1 VU: reads + 1 order, every saga followed to its end |
| `read-heavy` | GETs only (catalog, restaurant stock, order status) |
| `place-orders` | `POST /api/orders` only: the whole saga |
| `mixed` | 80% reads, 20% orders |
| `stress` | ramps the mix until a threshold breaks: the knee |
| `spike` | base → peak in 10 s, hold, back down |
| `soak` | moderate mix for an hour: leaks, outbox growth, connection churn |
| `one-million` | exactly `TOTAL` (1,000,000) requests at `RATE` (2,000/s) |

A run fails when `http_req_failed` ≥ 1%, read p95 ≥ 500 ms, order p95 ≥ 1 s, any iteration is
dropped, or a sampled saga does not finish in time. `make load_verify` then checks that every
order is PAID or CANCELLED, no outbox message is FAILED or STARTED, the `.dlq` topics are empty,
and each seeded product's restaurant stock equals its start value minus the quantities of its
PAID orders. On the lab the scripts pause the CI runner during a run.

### Findings so far

- Everything shares one 8-core machine, so the numbers show **relative** bottlenecks, not
  production capacity.
- Order placement is the expensive path: at least 4 database transactions and 4 Kafka messages
  per order across 3 services. The saga backlog (consumer lag, `STARTED` outbox rows) grows long
  before HTTP latency does.
- On kind, `mixed` at 150 req/s passed (p95 15 ms, saga p95 3.9 s). At 400 req/s with KEDA going
  from 2 to 3 replicas, HTTP stayed fast enough (p95 310 ms) but the saga consumers fell behind
  at 80 orders/s: saga p95 53 s, max lag 4,888, and the run failed on saga timeouts. Afterwards
  the shop drained and `make load_verify` passed (21,634 orders PAID, nothing lost).
- Still to record: `one-million` on the lab, 1 vs. 3 replicas, kind vs. lab, per-pod skew, a pod
  killed mid-run, a worker halted mid-run.

### Micro-benchmarks

```bash
cd shared && cargo bench --features contracts --bench avro      # Avro encode/decode of a saga message
cd order && cargo bench -p application --bench create_order     # command -> order + price validation
```

Encoding a `RestaurantApprovalRequest` takes ~50 µs and decoding it ~45 µs, of which ~35 µs is
parsing the schema, which `AvroMessage` does on every call. Mapping and validating an order
takes a few µs: noise next to the database and Kafka round trips.

---

## 12. Configuration

Environment variables (a service also reads its `.env` file):

| Variable | Default | Used by |
|---|---|---|
| `ENVIRONMENT` | `development` | services; returned by `/api/info` |
| `HTTP_URL` | `127.0.0.1:<port>` (gateway `0.0.0.0:8070`) | all |
| `DATABASE_URL` | `postgres://postgres:postgres@localhost:5432/<service>_db` | services |
| `DB_MAX_CONNECTIONS` | `16` (Kubernetes: `10`) | services, see [Database connections](#database-connections) |
| `DB_CONNECTION_TIMEOUT_MS` | `5000` | services |
| `KAFKA_HOST` | `localhost:9092` | services (comma-separated brokers) |
| `KAFKA_GROUP_ID` | `order-service-group` | order (restaurant and payment use fixed groups) |
| `OUTBOX_POLL_INTERVAL_MS` | `1000` | order, restaurant, payment |
| `OUTBOX_BATCH_SIZE` | `100` | order, restaurant, payment |
| `OUTBOX_MAX_ATTEMPTS` | `10` | order, restaurant, payment |
| `OUTBOX_RETENTION_HOURS` | `24` | order, restaurant, payment |
| `RUST_LOG` | `info` | all |
| `LOG_FORMAT` | unset (readable) | all; `json` for one JSON object per line |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | unset (no traces) | all; OTLP/HTTP collector, e.g. `http://alloy.monitoring:4318` |
| `OTEL_TRACES_SAMPLER`, `OTEL_TRACES_SAMPLER_ARG` | `parentbased_always_on` | all; e.g. `parentbased_traceidratio` + `0.1` |
| `OTEL_SERVICE_NAME` | the service's name | all |
| `MICROSERVICE_<NAME>_URL` | `http://localhost:<port>` | api-gateway (one per service) |
| `CONNECT_TIMEOUT_MS` / `RESPONSE_TIMEOUT_MS` | `1000` / `10000` | api-gateway |
| `POOL_IDLE_TIMEOUT_MS` / `POOL_MAX_IDLE_PER_HOST` | `30000` / `32` | api-gateway |
| `CB_SLIDING_WINDOW_SIZE`, `CB_FAILURE_RATE_THRESHOLD`, `CB_PERMITTED_CALLS_IN_HALF_OPEN`, `CB_WAIT_IN_OPEN_MS` | `10`, `50`, `2`, `10000` | api-gateway |

`SERVICE_NAME` is also read (set in `.env.example` and the chart) but not used for anything at
the moment.

Script and Makefile variables: `TARGET=kind|lab`, `KIND_HTTP_PORT` (8088), `KIND_KAFKA_PORT`
(9093), `SERVICES`, `SKIP_BUILD` (kind), `SCENARIO`, `PARALLELISM`, `RATE`, `DURATION`, `MIX`,
`TOTAL`, `DIRECT` (load tests, see [load-tests/README.md](load-tests/README.md)).

---

## 13. Tests

```bash
make test_all                                        # 5 services + gateway: unit + integration tests (needs Docker)
cd order && cargo test --workspace --lib --bins      # unit tests only
cd order && cargo test -p infrastructure --tests     # Postgres integration tests
cd order && cargo test -p starter --test order_saga  # the saga against real Postgres + Kafka
cd shared && cargo test --all-features               # shared, incl. db, kafka and outbox integration tests
make e2e                                             # whole system, against the running compose stack
make kind_e2e                                        # the same through the gateway on kind
```

What is covered:

- **Unit tests:** domain state machines and invariants, use cases and saga steps (with
  `mockall` mocks of the ports), the outbox relay, Avro encoding and decoding of every message,
  mappers, and the HTTP endpoints (actix test server with mocked repositories).
- **OpenAPI tests** fail when a registered route, operation id, response code or schema is
  missing from the generated document.
- **Integration tests** use [testcontainers](https://rust.testcontainers.org/) to start
  throwaway **Postgres** and **Kafka** containers:
  - repositories, including the atomicity of the saga/outbox transactions and the
    compare-and-set status update;
  - `shared`: the pool and migrations, the Kafka consumer group and DLQ, two relays on one
    outbox table without duplicates;
  - order (`starter/tests/order_saga.rs`): the saga runs inside the real service, with the real
    relay and Kafka listener; the restaurant and payment replies are produced to Kafka. It covers
    the happy path (`PAID`, including a duplicated approval), the rejection and the payment
    failure (`CANCELLED`, with the restaurant cancel request in the outbox);
  - restaurant and payment: their relays publish real replies to Kafka, which the tests consume
    and decode.
- **End-to-end** (`scripts/e2e-saga.sh`) drives the running services over HTTP: a paid order, an
  order the restaurant rejects for lack of stock, and the compensation path. For the last one it
  stops the payment service, answers the payment request with a `FAILED` reply
  (`shared/examples/fail_payment.rs`), and checks that the order is cancelled and the restaurant's
  stock and approval are restored / `CANCELLED`. `SKIP_COMPENSATION=1` skips that scenario;
  `BASE_URL` routes everything through the gateway and `TARGET=k8s` stops payment with
  `kubectl scale`.

Each service's `Makefile` also wraps the Diesel CLI (`migration`, `migrate_up`, `migrate_down`,
`print_schema`).

---

## 14. Project layout

```
rust-microservices/
├── catalog/ customer/ order/ restaurant/ payment/   one Cargo workspace per service (src/<layer>/)
├── shared/                the `shared` crate: src/, avro/schemas/, tests/, benches/, examples/
├── api-gateway/           the Rust (axum) API gateway
├── .github/workflows/     ci.yml, release.yml, infra.yml (GitHub and Forgejo)
├── scripts/
│   ├── e2e-saga.sh        end-to-end saga test
│   └── ci/changed-crates.sh
├── docker-compose/
│   ├── development/       hot-reload stack, Kafka topic definitions
│   └── production/        release images, docker-compose.scale.yml
├── deploy/                Kubernetes: Helm charts, platform values, Argo CD apps, kind + scripts
├── infra/                 homelab: lab.yml, Vagrantfile, Ansible, scripts (see infra/README.md)
├── load-tests/            k6 scenarios, k6-operator TestRun, run/verify scripts (see load-tests/README.md)
├── Makefile               every entry point: compose, kind, lab, load tests
└── .cargo/config.toml     dev profile: less debug info for dependencies
```

Tools the scripts download (kind, k6, kubeseal, Ansible) go to `.tools/`. Load-test reports go
to `load-tests/reports/`. Neither is in git.

---

## Homelab verification results

Run on 2026-10-20 against the lab (4 VMs on one 8-core, 32 GB laptop), services on images
`1.1.0-lab`, then `1.2.0-lab` for order and restaurant (the fixes below). The per-run numbers are
in [load-tests/README.md](load-tests/README.md#runs).

**Load test volume:** 100,000 requests instead of the planned 1,000,000. The total only sets how
long a run lasts (`TOTAL / RATE`); the rate is what the laptop can't hold. Under load the VMs see
25-60% CPU steal, and the saga consumers handle one message at a time per pod, so the lab sustains
about 10 orders/s. The runs use `TOTAL=100000 RATE=150 MIX=0.95` (7.5 orders/s, about 11 minutes).

| Check | Result |
|---|---|
| Trace → logs | A slow order trace (3.4 s, 4 services) opens the logs of order, restaurant and payment by trace id; Loki → Tempo works the other way. The api-gateway logs only failures, so a successful request has no gateway lines. |
| Alerts on ntfy | Fired and resolved: `ShopConsumerLagGrowing` (payment paused via KEDA), `ShopOutboxStuck` and `ShopKafkaDown` (Kafka's node halted), `LabNodeNotReady`, `ShopPostgresDown`, `ShopCircuitBreakerOpen`, `ShopDbPoolExhausted`, `ShopHighErrorRate`. |
| Outbox publish times | 99% of publishes take 5-100 ms. The ~520 ms ones only happen right after a pod starts: the first send per producer and topic fetches metadata and a producer id. |
| Per-pod balance, 3 replicas | Kafka consumers within ±3% (partitions split evenly). HTTP per pod: api-gateway within ±2%. catalog, order and restaurant vary between runs from ±10% to 1.5× max/min: the gateway keeps a few keep-alive connections per upstream and Cilium balances connections, not requests. |
| Pod killed mid-run | 1 failed request of 92,153 (0.001%) with a catalog pod deleted and an order pod force-killed. |
| KEDA | Scaled api-gateway, catalog, order and restaurant 2 → 4 under load and back to 2 after the cooldown, twice. |
| 100k run, 3 replicas | 100,427 requests, 0 failed, p50 14 ms, p95 56 ms, p99 111 ms, saga p95 3.7 s, max lag 63; `make load_verify` passes. |
| 100k run, 1 replica | 100,655 requests, 1 failed, p95 41 ms, saga p95 4.6 s, max lag 50; `make load_verify` passes. At this rate the host's shared CPU, not the replica count, is the limit. |
| Node lost mid-run | `vagrant halt k8s-w2` at 50 req/s: Kafka, order-db and half the stateless pods were on it. The second Envoy pod kept the entry point up; reads served by w1 kept working, orders failed (50.9% of the run's requests) until the node returned. Stateless pods moved to w1 after the 5-minute eviction delay. Kafka and order-db came back with the node (local volumes), all pods were ready 3 minutes after it booted, every order ended PAID and the stock was consistent. |
| Live load test dashboard | Every panel showed the run (client load, services, pools, lag, outbox, sagas, node steal) once the Replicas panels read the Deployments. |

**Found and fixed during the checks:**

- Restaurant stock lost updates with several replicas: an approval wrote back the absolute
  quantity it computed, so two replicas approving the same product at once undid each other's
  decrease (49 of 50 products off after a 3-replica run). Stock changes are now applied relative to
  the stored quantity.
- `order_items` had no index on `order_id`: every order lookup scanned the table, and order's
  connection pool ran out at 150 req/s.
- Strimzi rolled the only Kafka broker when a metrics config update failed under load, taking
  Kafka down for every service: the Strimzi metrics reporter's allowlist is a dynamic broker
  config that Kafka reads back as `null`, so every reconciliation re-applied it. Broker metrics now
  come from the JMX exporter.
- The kafka-exporter took ~10 s per scrape (the default timeout), so the lag series kept
  disappearing and the lag alert flapped; it now has a 25 s timeout and skips Kafka's internal
  topics.
- Tempo was OOM-killed at its 1 GiB limit with every request traced: the lab traces one in ten.
- `LabCiRunnerOffline` fired during long load tests (`make lab_quiet` stops the runner on purpose).
- k6 `setup()` timed out right after `pin_replicas` while the consumers rebalanced; the stress
  scenario stopped on a single dropped iteration.

---

## Known limitations and next steps

- **catalog, customer and the manual payment endpoints publish directly** to Kafka after
  committing (no outbox). A publish failure returns HTTP 500 but keeps the change, which leaves
  the order and restaurant copies behind until the next update of that record.
- **Saga throughput is one message at a time per consumer pod.** A pod's listener handles its
  partitions sequentially, so the saga rate is about replicas ÷ handler time (on the lab ~10
  orders/s, as handlers slow from ~95 ms to 400+ ms under CPU steal). Handling partitions in
  parallel within a pod would raise it without more replicas.
- **Consumers dead-letter during infrastructure outages.** A message is retried 5 times (~30 s)
  before it goes to `<topic>.dlq`, shorter than a node outage. When k8s-w2 was halted, one saga
  reply was dead-lettered; it came back only because the offset commit was lost too (the order saga
  finished, the DLQ keeps a stale copy and `load_verify` reports it). Retrying without limit, or
  pausing the partition, while the database or Kafka is unavailable would keep such messages.
- **Order item prices are not checked against the catalog.** The order service checks that the
  client's prices add up, not that they match the replicated product prices.
- **The catalog quantity check is per item.** Two lines of the same product are each compared
  with the catalog quantity; the restaurant does sum them.
- **Single instances:** one Kafka broker and one Postgres instance per service. Postgres volumes
  are local-path volumes, so on the lab a database pod is tied to its node. Losing a worker stops
  the databases on it until it returns.
- **Schema parsing on every Avro call** costs ~35 µs per message; caching the parsed schemas
  would cut the serialization cost by about two thirds.
- **Sealed Secrets key backup:** the controller's private key is not part of `make lab_backup`
  yet; rebuilding the cluster means re-sealing the secrets (`~/.lab/grafana-admin.env`).
- **CI metrics:** Forgejo exposes no job queue or job duration metrics, so the infrastructure
  dashboard only shows whether the runner runs; the Actions page shows the jobs.
- **Linkerd** (per-request east-west balancing) is not installed. The lab measured more than 20%
  HTTP skew for some services (connection-level balancing of the gateway's keep-alive
  connections), the plan's condition for adding it, but a proxy per pod doesn't fit the lab's
  memory (the host already swaps under load). Kafka consumption, the bulk of the saga work, is
  balanced by partition and unaffected.
- **The lab is CPU-bound:** 4 VMs share 8 cores with k6, and under load they see 25-60% CPU steal;
  the load tests run 100,000 requests at 150 req/s rather than 1,000,000 at 2,000.
