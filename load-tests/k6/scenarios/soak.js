// Moderate load for a long time, for leaks, outbox growth and connection churn: RATE (200)
// requests/s of the mixed traffic for DURATION (1h).
import { env, num } from '../lib/config.js';
import { mixedScenarios } from '../lib/mix.js';
import { loadShop } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

export const options = {
  scenarios: mixedScenarios({ rate: num('RATE', 200), duration: env('DURATION', '1h'), mix: num('MIX', 0.8) }),
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export { reads, orders } from '../lib/mix.js';
