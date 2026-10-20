// A sudden burst: the mixed traffic jumps from BASE_RATE (50) to PEAK_RATE (1500) requests/s in 10 s,
// holds it for HOLD (1m), drops back and stays at the base for 2 minutes while the backlog drains.
import { env, num } from '../lib/config.js';
import { mixedRampingScenarios } from '../lib/mix.js';
import { loadShop } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

const BASE = num('BASE_RATE', 50);
const PEAK = num('PEAK_RATE', 1500);

export const options = {
  scenarios: mixedRampingScenarios({
    startRate: BASE,
    stages: [
      { target: BASE, duration: '30s' },
      { target: PEAK, duration: '10s' },
      { target: PEAK, duration: env('HOLD', '1m') },
      { target: BASE, duration: '10s' },
      { target: BASE, duration: '2m' },
    ],
    mix: num('MIX', 0.8),
  }),
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export { reads, orders } from '../lib/mix.js';
