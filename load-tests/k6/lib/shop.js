import { sleep } from 'k6';
import exec from 'k6/execution';
import { Counter, Trend } from 'k6/metrics';

import { num } from './config.js';
import { data, get, post } from './http.js';

// The seeded test data (seed.js) is recognisable by this prefix.
export const PREFIX = 'k6-';

// From POST /api/orders until the order is PAID or CANCELLED, for the sampled orders.
const sagaDuration = new Trend('saga_e2e_duration', true);
const sagaTimeouts = new Counter('saga_timeouts');

// The catalog formats prices as money ("$10.00").
const amount = (price) => Number(String(price).replace(/[^0-9.]/g, ''));

// The seeded products (catalog ids), for setup().
export function seededProducts() {
  return (data(get('catalog', '/api/products', 'seed', '/api/products')) || [])
    .filter((p) => p.title.startsWith(PREFIX))
    .map((p) => ({ id: p.id, price: amount(p.price) }));
}

// Creates `count` customers and waits until the order service replicated each of them (it
// rejects an order of an unknown customer). Returns their ids.
export function createCustomers(count, products) {
  const ids = [];
  const run = Date.now().toString(36);
  for (let i = 0; i < count; i++) {
    const customer = data(post('customer', '/api/customers',
      { user_name: `${PREFIX}${run}-${i}`, first_name: 'Load', last_name: `Tester ${i}` },
      'seed', '/api/customers'));
    if (customer) {
      ids.push(customer.id);
    }
  }
  for (const id of ids) {
    if (!waitUntil(() => placeOrder({ customers: [id], products }, { kind: 'seed', sample: 0, probe: true }), 60)) {
      exec.test.abort(`the order service didn't learn about customer ${id} within 60 s`);
    }
  }
  return ids;
}

// setup() of the scenarios: the seeded products, CUSTOMERS (20) new customers, and with `orders`
// > 0 that many orders, so the read path has tracking ids to look up.
export function loadShop({ orders = 0 } = {}) {
  const products = seededProducts();
  if (products.length === 0) {
    exec.test.abort('no seeded products: run `make load_seed` first');
  }
  const shop = { customers: createCustomers(num('CUSTOMERS', 20), products), products, trackingIds: [] };
  for (let i = 0; i < orders; i++) {
    const trackingId = placeOrder(shop, { kind: 'seed', sample: 0 });
    if (trackingId) {
      shop.trackingIds.push(trackingId);
    }
  }
  return shop;
}

// Calls `attempt` every 2 s until it returns something truthy, for up to `seconds`.
export function waitUntil(attempt, seconds) {
  const deadline = Date.now() + seconds * 1000;
  for (;;) {
    const result = attempt();
    if (result || Date.now() >= deadline) {
      return result;
    }
    sleep(2);
  }
}

const pick = (items) => items[Math.floor(Math.random() * items.length)];

// One GET of the read path: a product page (catalog), its stock (restaurant), the product
// list, an order's status. (The customer service has no read endpoint.)
export function readOnce(shop) {
  const roll = Math.random();
  if (roll < 0.4 || (roll >= 0.75 && shop.trackingIds.length === 0)) {
    get('catalog', `/api/products/${pick(shop.products).id}`, 'read', '/api/products/{id}');
  } else if (roll < 0.6) {
    get('restaurant', `/api/products/${pick(shop.products).id}`, 'read', '/api/products/{id}');
  } else if (roll < 0.75) {
    get('catalog', '/api/products', 'read', '/api/products');
  } else {
    get('order', `/api/orders/${pick(shop.trackingIds)}`, 'read', '/api/orders/{tracking_id}');
  }
}

// Places an order of 1 item. A `sample` share of the orders (SAGA_SAMPLE, default 1%) is then
// followed until its saga finished, for saga_e2e_duration. Returns the tracking id.
export function placeOrder(shop, { kind = 'order', sample = num('SAGA_SAMPLE', 0.01), probe = false } = {}) {
  const product = pick(shop.products);
  const started = Date.now();
  const response = post(
    'order',
    '/api/orders',
    {
      customer_id: pick(shop.customers),
      price: product.price,
      items: [{ product_id: product.id, quantity: 1, price: product.price, sub_total: product.price }],
    },
    kind,
    '/api/orders',
    { probe },
  );
  const trackingId = (data(response) || {}).tracking_id;
  if (trackingId && Math.random() < sample) {
    followSaga(trackingId, started);
  }
  return trackingId;
}

// Polls the order until PAID or CANCELLED (or SAGA_TIMEOUT_S); returns the final status.
export function sagaOutcome(trackingId, kind = 'saga-poll') {
  const deadline = Date.now() + num('SAGA_TIMEOUT_S', 60) * 1000;
  const interval = num('SAGA_POLL_MS', 250) / 1000;
  while (Date.now() < deadline) {
    sleep(interval);
    const status = (data(get('order', `/api/orders/${trackingId}`, kind, '/api/orders/{tracking_id}')) || {}).status;
    if (status === 'PAID' || status === 'CANCELLED') {
      return status;
    }
  }
  return undefined;
}

function followSaga(trackingId, started) {
  const status = sagaOutcome(trackingId);
  if (status) {
    sagaDuration.add(Date.now() - started, { outcome: status });
  } else {
    sagaTimeouts.add(1);
  }
}
