// Sanity check: 1 VU for DURATION (1m) reads, places an order and follows every saga to its end.
import { sleep } from 'k6';

import { env } from '../lib/config.js';
import { loadShop, placeOrder, readOnce } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

export const options = {
  scenarios: { smoke: { executor: 'constant-vus', vus: 1, duration: env('DURATION', '1m') } },
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 3 });

export default function (shop) {
  for (let i = 0; i < 3; i++) {
    readOnce(shop);
  }
  placeOrder(shop, { sample: 1 });
  sleep(1);
}
