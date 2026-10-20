// Creates the products the scenarios order, once per cluster (re-running it only fills gaps):
// PRODUCTS (50) products in one category with a stock of STOCK each, so the restaurant never
// rejects an order for lack of stock. Ends when the restaurant and the order service replicated
// every product: a probe order per product must end PAID. (Each run creates its own customers.)
//   make load_seed [TARGET=lab]
import exec from 'k6/execution';

import { num } from './lib/config.js';
import { data, get, post } from './lib/http.js';
import { PREFIX, createCustomers, placeOrder, sagaOutcome, seededProducts, waitUntil } from './lib/shop.js';

const PRODUCTS = num('PRODUCTS', 50);
// The stock verify.sh checks against: STOCK - the quantities of every PAID order.
const STOCK = num('STOCK', 1000000000);
const PRICE = 10;

// Probes of products the services haven't replicated yet fail on purpose (tagged as probes), so
// the thresholds only cover the rest.
export const options = {
  scenarios: { seed: { executor: 'shared-iterations', vus: 1, iterations: 1, maxDuration: '30m' } },
  thresholds: { http_req_failed: ['rate<0.01'] },
};

export default function () {
  let category = (data(get('catalog', '/api/categories', 'seed', '/api/categories')) || [])
    .find((c) => c.title === `${PREFIX}load-test`);
  if (!category) {
    category = data(post('catalog', '/api/categories',
      { title: `${PREFIX}load-test`, description: 'Load test products' }, 'seed', '/api/categories'));
  }
  if (!category) {
    exec.test.abort('could not create the category');
  }

  const existing = seededProducts().length;
  for (let i = existing; i < PRODUCTS; i++) {
    post('catalog', '/api/products', {
      title: `${PREFIX}product-${i}`,
      description: 'Load test product',
      quantity: STOCK,
      price: PRICE,
      category_id: category.id,
    }, 'seed', '/api/products');
  }
  const products = seededProducts();
  console.log(`${products.length} products (${products.length - existing} new)`);

  // The restaurant and the order service learn about products through Kafka: an order of each
  // product must be accepted (order service) and approved (restaurant), ending PAID.
  const customers = createCustomers(1, products.slice(0, 1));
  for (const product of products) {
    const shop = { customers, products: [product] };
    const paid = waitUntil(() => {
      const trackingId = placeOrder(shop, { kind: 'seed', sample: 0, probe: true });
      return trackingId && sagaOutcome(trackingId, 'seed') === 'PAID';
    }, 120);
    if (!paid) {
      exec.test.abort(`product ${product.id} wasn't replicated within 2 minutes`);
    }
  }
  console.log('every product is replicated: the shop is ready for load');
}
