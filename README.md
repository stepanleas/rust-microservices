# Rust Microservices

A food-ordering system built as five Rust microservices (actix-web, Diesel/PostgreSQL, Kafka/Avro)
following a hexagonal architecture. Placing an order runs an **orchestrated saga**: the order
service drives the restaurant approval and the payment, and every message between services is sent
through a **transactional outbox**.

| Service    | Port | Database (host port)     | Responsibility                                                                    |
|------------|------|--------------------------|-----------------------------------------------------------------------------------|
| catalog    | 8080 | `catalog_db` (5432)      | Categories and products. Publishes `product-*` events.                            |
| order      | 8081 | `order_db` (5433)        | Places orders, **orchestrates the saga**, exposes order tracking.                 |
| customer   | 8082 | `customer_db` (5434)     | Customers. Publishes `customer-*` events.                                         |
| restaurant | 8083 | `restaurant_db` (5435)   | Approves or rejects orders against its own product stock.                         |
| payment    | 8084 | `payment_db` (5436)      | Takes the payment for approved orders (a local record, no payment provider).      |
| api-gateway | 8070 | –                       | Single entry point: `/shop/<service>/**` → `<service>/**`, correlation id, circuit breaker. |

Every service has Swagger UI at `http://localhost:<port>/swagger-ui/` and the raw OpenAPI document
at `http://localhost:<port>/api-docs/openapi.json`.

---

## How an order flows through the system

### Reference data (replicated through Kafka)

Before an order can be placed, the order and restaurant services need local copies of the data they
validate against. They build them by consuming events:

```
customer ──customer-created/updated──▶ order
catalog  ──product-created/updated/deleted──▶ order, restaurant
```

* The **order** service checks that the customer exists and that enough quantity is listed in the
  catalog.
* The **restaurant** service keeps its own **stock** per product. Every approved order lowers it,
  so the restaurant can reject an order even though the catalog still lists the product. A later
  `product-updated` event resets the stock to the catalog quantity.

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
        else payment FAILED
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

1. **Place the order** – `POST /api/orders` validates the customer, products, quantities and that
   the prices add up. In a single database transaction the order service stores the order as
   `PENDING`, creates an `order_sagas` row with status `STARTED`, and writes a
   `RESTAURANT_APPROVAL_REQUEST` row into `outbox_messages`. The client immediately gets a
   `tracking_id`.
2. **Ask the restaurant** – the order service's outbox relay picks up the row, publishes it to the
   `restaurant-approval-request` topic and marks it `COMPLETED`.
3. **Restaurant decides** – the restaurant checks every product against its stock. It stores an
   `order_approvals` row (`APPROVED` or `REJECTED` with failure messages), lowers the stock when
   approved, and writes a `RESTAURANT_APPROVAL_RESPONSE` outbox row, all in one transaction. Its
   relay publishes the reply to `restaurant-approval-response`.
4. **Order confirms** – the order service consumes the reply:
   * **approved:** order → `APPROVED`, saga → `PROCESSING`, and a `PAYMENT_REQUEST` outbox row is
     written in the same transaction. The relay publishes it to `payment-request`.
   * **rejected:** order → `CANCELLED` with the restaurant's failure messages, saga → `FAILED`.
     The saga ends here and no payment is requested.
5. **Payment** – the payment service inserts a row into its `payments` table (`COMPLETED` for a
   positive amount, otherwise `FAILED`) and an outbox row in the same transaction. Its relay
   publishes the reply to `payment-response`.
6. **Order completes** – the order service consumes the reply:
   * **completed:** order → `PAID`, saga → `SUCCEEDED`.
   * **failed:** order → `CANCELLED` with the payment failure messages, saga → `FAILED`, and a
     `RESTAURANT_APPROVAL_CANCEL_REQUEST` outbox row is written in the same transaction (see
     [Compensation](#compensation-payment-failed-after-approval)).

Track progress with `GET /api/orders/{tracking_id}`.

### State machines

Order status (order service, `orders.status`):

```
PENDING ──restaurant approved──▶ APPROVED ──payment completed──▶ PAID
   │                                 │
   └──restaurant rejected──▶ CANCELLED ◀──payment failed──┘
```

Saga status (order service, `order_sagas.status`):

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

Illegal transitions (for example paying an order the restaurant hasn't approved yet) are rejected
by the domain aggregates (`Order::approve/pay/cancel`, `OrderSaga::restaurant_approved/payment_completed/fail`,
`OrderApproval::cancel`).

### Compensation (payment failed after approval)

When the restaurant approved an order it has already lowered its stock. If the payment then fails,
the order service undoes that step:

1. In the transaction that cancels the order and fails the saga, the order service writes a
   `RESTAURANT_APPROVAL_CANCEL_REQUEST` outbox row with the order's products and quantities. It
   does so only when the failure arrives in saga status `PROCESSING`, i.e. after the restaurant
   approved; a rejected order has nothing to give back.
2. The relay publishes it to `restaurant-approval-cancel-request`, keyed by the order id.
3. The restaurant looks up the approval of that order. If it is `APPROVED`, it adds the requested
   quantities back to its stock and marks the approval `CANCELLED`, in one transaction. A missing,
   `REJECTED` or already `CANCELLED` approval is logged and ignored, so redeliveries are harmless.
   Products that no longer exist are skipped with a warning. No reply is sent.

`GET /api/order-approvals/{order_id}` then shows `CANCELLED`.

### Topics and message contracts

All saga messages are Avro object containers with the schema embedded. The message key is the
order id, so every message of one order lands on the same partition.

The schemas and the Rust models live in one place, the `shared` crate (`contracts` feature), so
producer and consumer can't drift apart. Topic names are constants in `shared::contracts::topics`.

| Topic                          | Producer → consumer   | Schema (`shared/avro/schemas/`)                 |
|--------------------------------|-----------------------|-------------------------------------------------|
| `restaurant-approval-request`  | order → restaurant    | `restaurant_approval_request_avro_model.avsc`   |
| `restaurant-approval-response` | restaurant → order    | `restaurant_approval_response_avro_model.avsc`  |
| `restaurant-approval-cancel-request` | order → restaurant | `restaurant_approval_cancel_request_avro_model.avsc` |
| `payment-request`              | order → payment       | `payment_request_avro_model.avsc`               |
| `payment-response`             | payment → order       | `payment_response_avro_model.avsc`              |
| `customer-created/updated`     | customer → order      | `customer_*_event_avro_model.avsc`              |
| `product-created/updated/deleted` | catalog → order, restaurant | `product_*_event_avro_model.avsc`     |
| `payment-created/updated`      | payment → –           | `payment_*_event_avro_model.avsc`               |

Every saga message carries `saga_id` and `order_id`. Prices are decimal strings (`"60.00"`),
statuses are upper-case strings (`APPROVED`/`REJECTED`, `COMPLETED`/`FAILED`/`CANCELLED`), and
`failure_messages` is an array of strings. The topics are created by the `kafka-init` container
(`docker-compose/*/kafka/topics/*.json`). Brokers also auto-create topics, and the services ask for
their topics explicitly at start-up, so a fresh broker works too.

---

## Transactional outbox

A service must never change its database and then publish to Kafka as two independent steps. If it
crashes in between, or Kafka is down, the other services never hear about the change. The order,
restaurant and payment services therefore use the **transactional outbox** pattern:

1. The business change and the message to send are written in **the same database transaction**.
   The message goes to the `outbox_messages` table:

   ```sql
   CREATE TABLE outbox_messages (
       id uuid PRIMARY KEY,
       saga_id uuid NOT NULL,
       message_type VARCHAR(64) NOT NULL,     -- e.g. RESTAURANT_APPROVAL_REQUEST, PAYMENT_REQUEST
       payload JSONB NOT NULL,                -- message body, mapped to Avro when published
       outbox_status VARCHAR(16) NOT NULL,    -- STARTED | COMPLETED | FAILED
       attempts INT NOT NULL DEFAULT 0,
       created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
       processed_at TIMESTAMP
   );
   ```

2. An **outbox relay** thread (`OutboxRelay::process_batch`) polls every
   `OUTBOX_POLL_INTERVAL_MS`. In one transaction it claims up to `OUTBOX_BATCH_SIZE` `STARTED`
   rows, oldest first, with `SELECT … FOR UPDATE SKIP LOCKED`, publishes each one to its topic,
   marks it `COMPLETED` and commits. Every replica runs a relay; rows claimed by one relay are
   skipped by the others, so scaling out never sends a message twice.
3. If publishing fails, the row stays `STARTED` and `attempts` is incremented, so the next tick
   retries it. After `OUTBOX_MAX_ATTEMPTS` failures the row is marked `FAILED` and an error is
   logged.
4. Every 5 minutes the relay deletes `COMPLETED` rows processed more than
   `OUTBOX_RETENTION_HOURS` ago (in batches of 5,000), so the table stays small under load.

This gives **at-least-once** delivery, so every consumer is **idempotent**:

* **order** only applies a reply while the saga is still in the expected status (`STARTED` for
  restaurant replies, `PROCESSING` for payment replies). Duplicate or late replies are logged and
  ignored. The status update itself is a compare-and-set (`UPDATE … WHERE status = expected`), so
  two concurrent deliveries can't both win.
* **restaurant** stores at most one approval per order (`order_approvals.order_id` is unique) and
  skips requests it has already answered.
* **payment** stores at most one payment per order (`payments.order_id` is unique) and skips
  requests it has already processed.

### Consuming: retries and dead letter topics

Every consuming service runs one listener thread (`shared::kafka::listen`) with one handler that
dispatches by topic. The listener is a member of the service's consumer group (`KAFKA_GROUP_ID`),
so the replicas of a service split the topics' partitions between them (cooperative-sticky
rebalancing) instead of each reading every message. For each message:

1. The handler is called. Handlers return errors instead of logging and swallowing them; an
   unknown topic or an undecodable message is an error too.
2. A failing message is retried with exponential backoff (`RetryPolicy::default()`: 5 attempts,
   500 ms doubling up to 10 s), so a short database outage does not lose a saga reply. Messages of
   one partition are handled in order, so a retried message holds back the ones behind it.
3. After the last attempt the message is published unchanged (same key) to `<topic>.dlq`, for
   example `payment-response.dlq`, and the listener moves on. The dead letter topics are
   auto-created by the broker.
4. The message's offset is stored only once it was handled or dead-lettered, and only stored
   offsets are committed (every second and on rebalance), so a crash causes redelivery, never loss.

A duplicate saga reply for a saga that has moved on is a successful no-op, not an error, so it
never ends up in a dead letter topic.

The Kafka client is librdkafka (`rdkafka`, built from source, so building needs a C toolchain,
`make` and `perl`). Producers connect lazily and are idempotent with `acks=all`; `send` waits for
the broker's acknowledgement, so the outbox relays mark a message completed only once Kafka has it.
A send that cannot be delivered within 10 s fails and the relay retries the row on its next poll.
Producers also accept record headers (`send_with_headers`), used to propagate trace context.

---

## Project layout

```
rust-microservices/
├── catalog/ customer/ order/ payment/ restaurant/   one Cargo workspace per service
├── shared/          the `shared` crate (2.0.0) used by every service
├── api-gateway/     the Rust (axum) API gateway
├── .github/workflows/   CI (fmt, clippy, tests per changed crate) and per-service image releases
├── docker-compose/  development and production stacks, Kafka topic definitions
└── scripts/         end-to-end saga test
```

Each service has the same hexagonal layout:

```
<service>/src/
├── domain/          aggregates/<aggregate>/ (entity, builder, tests), events, domain errors (no I/O)
├── application/
│   ├── ports/in/    driving ports: the message-listener traits the messaging adapter calls
│   ├── ports/out/   driven ports: repositories, publishers (outbox ports come from shared)
│   ├── use_cases/   one synchronous struct per use case: `execute(..) -> AppResult<T>`
│   └── saga/, outbox.rs   order's saga steps; the service's outbox message types
├── infrastructure/  postgres/<aggregate>.rs (Diesel repository + rows), schema, migrations
├── messaging/       one Kafka MessageHandler (dispatch by topic), contract mappers, publishers
├── presentation/    actix-web controllers + OpenAPI docs, request/response DTOs, AppState
└── starter/         config (Settings) and wiring: DB pool, use cases, HTTP server,
                     Kafka listener thread, outbox relay thread
```

HTTP controllers only map DTOs and run a use case on the blocking thread pool
(`shared::web::blocking`), so Diesel and the Kafka producer never block the actix workers.

### The `shared` crate

`shared` is split into features so each layer only pulls in what it needs:

| Feature           | Contents                                                                      | Used by        |
|-------------------|-------------------------------------------------------------------------------|----------------|
| (none)            | `AppError`/`AppResult`, typed ids                                             | domain, application |
| `contracts`       | Avro schemas + models of every message, `topics`                              | messaging      |
| `outbox`          | generic `OutboxMessage`, outbox ports, `OutboxRelay`                          | application, messaging |
| `outbox-postgres` | `PostgresOutboxRepository`, `insert_outbox_message`                           | infrastructure |
| `db`              | `connect()`: r2d2 pool + embedded migrations                                  | infrastructure |
| `kafka`           | `KafkaProducer`, `KafkaConsumer`, `listen()` with retries and DLQ             | messaging      |
| `web`             | `ApiError`, `ErrorResponse`, `ValidatedJson`, correlation id middleware, `blocking()` | presentation |
| `mocks`           | mockall mocks of the shared ports                                             | tests          |

The services depend on it by path (`shared = { path = "../shared" }`), so a change to `shared` is
built and tested against every service in the same commit; there is no separate version to tag.
Because of that, the Docker images are built with the repository root as context:

```bash
docker build -f order/Dockerfile -t artellas/rust-order:1.4.0 .
```

---

## CI/CD

GitHub Actions runs from the root `.github/workflows/`:

* **`ci.yml`** (pushes to `main`, `feature/*`, `fix/*` and PRs to `main`) runs `cargo fmt --check`,
  `cargo clippy -D warnings` and `cargo test` for each crate whose directory changed. A change to
  `shared/` checks every crate.
* **`release.yml`** releases one service per tag `<service>-v<semver>`:

  ```bash
  git tag order-v1.4.0 && git push origin order-v1.4.0
  ```

  builds `order/Dockerfile` and pushes `artellas/rust-order:1.4.0`, `:1.4`, `:1` and `:latest` (needs the
  `DOCKER_USERNAME` / `DOCKER_PASSWORD` secrets of the `Rust Microservices` environment). An existing tag can also be released
  from **Actions → release → Run workflow** by entering one or more tags, e.g.
  `order-v1.0.0 payment-v1.0.0`; each tag is built in its own parallel job.

---

## Running

### Everything in Docker (development, hot reload)

```bash
make up                     # docker compose up -d --build for the 5 services, DBs and Kafka
make e2e                    # run the end-to-end saga scenarios against it
make down
```

The development containers compile with `cargo watch`, so the first start takes a few minutes.
Each service waits for its database and for `kafka-init` to have created the topics.

### Services on the host, infrastructure in Docker

```bash
make infra_up               # 5 Postgres containers + Kafka + topic creation
cd order && cargo run -p starter        # repeat for catalog, customer, restaurant, payment
```

Each service reads its `.env` file (copy `.env.example`); the defaults point to
`localhost:<db port>` and `localhost:9092`.

Kafka exposes two listeners: `kafka:29092` for containers and `localhost:9092` for the host.
Kafka runs as a single KRaft node (`apache/kafka` 4.x). Topics have 12 partitions, so up to 12 replicas
of a service share the load of its consumer group.

### Configuration

| Variable                  | Default                  | Used by                    |
|---------------------------|--------------------------|----------------------------|
| `ENVIRONMENT`             | `development`            | all                        |
| `HTTP_URL`                | per service              | all                        |
| `DATABASE_URL`            | per service              | all                        |
| `KAFKA_HOST`              | `localhost:9092`         | all                        |
| `KAFKA_GROUP_ID`          | `order-service-group`    | order (restaurant and payment use a fixed group) |
| `DB_MAX_CONNECTIONS`      | `16`                     | all (see [Scaling out](#scaling-out)) |
| `DB_CONNECTION_TIMEOUT_MS`| `5000`                   | all                        |
| `OUTBOX_POLL_INTERVAL_MS` | `1000`                   | order, restaurant, payment |
| `OUTBOX_BATCH_SIZE`       | `100`                    | order, restaurant, payment |
| `OUTBOX_MAX_ATTEMPTS`     | `10`                     | order, restaurant, payment |
| `OUTBOX_RETENTION_HOURS`  | `24`                     | order, restaurant, payment |
| `RUST_LOG`                | `info`                   | all                        |
| `LOG_FORMAT`              | unset (readable)         | all; `json` for one JSON object per line |
| `MICROSERVICE_<NAME>_URL` | `http://localhost:<port>` | api-gateway (one per service) |
| `CONNECT_TIMEOUT_MS` / `RESPONSE_TIMEOUT_MS` | `1000` / `10000` | api-gateway |
| `CB_SLIDING_WINDOW_SIZE`, `CB_FAILURE_RATE_THRESHOLD`, `CB_PERMITTED_CALLS_IN_HALF_OPEN`, `CB_WAIT_IN_OPEN_MS` | `10`, `50`, `2`, `10000` | api-gateway |

### Scaling out

Every service can run several replicas:

* **HTTP**: stateless. `GET /api/health/ready` answers 503 while the database is unreachable,
  so a load balancer or Kubernetes stops routing to that replica. `live` and `startup` never
  check dependencies.
* **Kafka consumers**: the replicas of a service form one consumer group and split the 12
  partitions of each topic, so up to 12 replicas share the work. A rebalance (replica added or
  stopped) may redeliver a few messages; the handlers are idempotent.
* **Outbox relays**: one per replica, coordinated by `FOR UPDATE SKIP LOCKED` (see above).
* **Graceful shutdown**: on SIGTERM the HTTP server stops accepting connections and finishes
  in-flight requests (up to 30 s). Then the Kafka listener finishes the message in hand and
  commits its offsets, and the relay finishes the message it is publishing and commits its batch
  (up to 15 s). A message still waiting for a retry is left for redelivery instead of being
  dead-lettered. In Kubernetes, set `terminationGracePeriodSeconds` to at least 50.
* **Database connections**: r2d2 opens all `DB_MAX_CONNECTIONS` connections of a replica at
  startup and keeps them. Every request, the Kafka listener and the relay borrow from the same
  pool, and the relay holds one connection for the duration of a batch. Keep

  ```
  replicas × DB_MAX_CONNECTIONS  <  max_connections − 3     (Postgres default: 100)
  ```

  For example, 3 replicas × 16 = 48 fits one default Postgres, while 6 × 16 = 96 does not. In
  Kubernetes the services use `DB_MAX_CONNECTIONS=10` and connect through PgBouncer, which then
  bounds the server-side connections instead.

### Logs

The services log through `tracing`. With `LOG_FORMAT=json` every line is a JSON object that
also carries the fields of the enclosing spans: `correlation_id` for HTTP requests (from the
`X-Correlation-ID` header, generated when missing), `topic`, `partition`, `offset` and `key` for
Kafka messages (the key is the order id on the saga topics), and `message_id`, `saga_id` and
`message_type` for outbox publishes.

---

## Trying it by hand

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

Ordering 2 more right afterwards is rejected by the restaurant (only 1 left in its stock). That
order ends `CANCELLED` with a failure message like
`Insufficient stock for product Margherita: requested 2, available 1`.

### Endpoints

| Service    | Endpoint                                     | Description                                   |
|------------|----------------------------------------------|-----------------------------------------------|
| order      | `POST /api/orders`                           | Place an order, starts the saga               |
| order      | `GET /api/orders/{tracking_id}`              | Order status, items and failure messages      |
| restaurant | `GET /api/order-approvals/{order_id}`        | The restaurant's decision for an order (`APPROVED`/`REJECTED`/`CANCELLED`) |
| restaurant | `GET /api/products`, `GET /api/products/{id}`| Restaurant stock (replicated from catalog)    |
| payment    | `GET /api/payments/{id}`                     | A payment                                     |
| payment    | `GET /api/payments/orders/{order_id}`        | The payment of an order                       |
| payment    | `POST /api/payments`, `PUT /api/payments/{id}` | Manual payment management                   |
| catalog    | `/api/categories`, `/api/products`           | CRUD                                          |
| customer   | `POST /api/customers`, `PUT /api/customers/{id}` | Create / update customers                 |
| all        | `GET /api/health/{startup,live,ready}`, `GET /api/info` | Health and info                    |

Every endpoint is also reachable through the gateway on port 8070 as `/shop/<service>/<path>`,
for example `curl localhost:8070/shop/order/api/orders/$TRACKING`.

Request/response schemas, examples and error codes are in each service's Swagger UI.

---

## Tests

```bash
make test_all               # every service: unit + integration tests (needs Docker)
cd order && cargo unit_tests            # unit tests only
cd order && cargo test -p infrastructure --tests   # Postgres integration tests
cd order && cargo test -p starter --test order_saga # saga against real Postgres + Kafka
make e2e                    # whole system, against running services
```

* **Unit tests** cover domain state machines, application handlers and saga steps (with
  `mockall` mocks of the ports), the outbox relay, Avro encoding/decoding of every message, and the
  HTTP endpoints (actix test server with mocked repositories).
* **OpenAPI tests** fail when a registered route, operation id, response code or schema is
  missing from the generated document.
* **Integration tests** use [testcontainers](https://rust.testcontainers.org/) to start throwaway
  **Postgres** and **Kafka** containers:
  * repositories, including atomicity of the saga/outbox transactions and the compare-and-set
    status update;
  * order: the saga runs inside the real service, with the real outbox relay and the real Kafka
    listener. The restaurant and payment replies are produced to Kafka. The happy path
    (`PAID`, including a duplicated approval), the rejection path and the payment failure path
    (`CANCELLED`, with the restaurant cancel request in the outbox) are covered;
  * restaurant and payment: their relays publish real replies to Kafka, which the tests consume
    and decode.
* **End-to-end** (`scripts/e2e-saga.sh`) drives all five running services over HTTP. It checks
  a paid order, an order the restaurant rejects for lack of stock, and the compensation path: it
  stops the payment service, answers the payment request with a `FAILED` reply
  (`shared/examples/fail_payment.rs`), and checks that the order is cancelled and the restaurant's
  stock and approval are restored/`CANCELLED`. Set `SKIP_COMPENSATION=1` to skip that scenario.

---

## Known limitations / next steps

* **catalog and customer publish directly** to Kafka after committing (no outbox yet). A publish
  failure there leaves the order/restaurant replicas behind until the next update.
* Outbox rows are kept after publishing. A periodic cleanup of old `COMPLETED` rows would bound
  the table size.
* The relay assumes one instance per service. Several instances would only cause duplicate
  sends, which the idempotent consumers absorb. To avoid them entirely, use
  `SELECT … FOR UPDATE SKIP LOCKED`.
