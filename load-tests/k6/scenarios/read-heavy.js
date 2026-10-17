// The read path only (HTTP + database reads, no saga): RATE (200) requests/s for DURATION (5m).
import { env, num, vusFor } from '../lib/config.js';
import { loadShop, readOnce } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

const RATE = num('RATE', 200);

export const options = {
  scenarios: {
    reads: { executor: 'constant-arrival-rate', rate: RATE, timeUnit: '1s', duration: env('DURATION', '5m'), ...vusFor(RATE) },
  },
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export default function (shop) {
  readOnce(shop);
}
