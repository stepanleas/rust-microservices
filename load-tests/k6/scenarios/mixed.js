// Realistic traffic: RATE (250) requests/s for DURATION (10m), a MIX (0.8) share reads and the rest
// orders.
import { env, num } from '../lib/config.js';
import { mixedScenarios } from '../lib/mix.js';
import { loadShop } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

export const options = {
  scenarios: mixedScenarios({ rate: num('RATE', 250), duration: env('DURATION', '10m'), mix: num('MIX', 0.8) }),
  thresholds: thresholds(),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export { reads, orders } from '../lib/mix.js';
