// The saga only: RATE (50) orders/s for DURATION (5m), each one 4 database transactions and 4 Kafka
// messages across order, restaurant and payment. SAGA_SAMPLE (1%) of them are followed to the end.
import { env, num, vusFor } from '../lib/config.js';
import { loadShop, placeOrder } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

const RATE = num('RATE', 50);

export const options = {
  scenarios: {
    orders: { executor: 'constant-arrival-rate', rate: RATE, timeUnit: '1s', duration: env('DURATION', '5m'), ...vusFor(RATE, { orders: true }) },
  },
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop();

export default function (shop) {
  placeOrder(shop);
}
