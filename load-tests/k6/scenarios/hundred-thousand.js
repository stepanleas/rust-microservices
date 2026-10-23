// Exactly TOTAL (100,000) requests at RATE (150) requests/s, so TOTAL / RATE seconds (11m07s): a MIX
// (0.95) share reads, the rest orders. Each of the two scenarios starts exactly its share of
// iterations (one request each), spread evenly over the run. On top of that come the GETs that
// follow SAGA_SAMPLE (1%) of the sagas (tag kind=saga-poll) and the setup's requests (kind=seed).
//
// PROFILE=max instead ramps the same mix up to MAX_RATE (5000) and stops at the first broken
// threshold, like stress.js; then the request count is whatever was reached.
import { env, num } from '../lib/config.js';
import { mixedRampingScenarios, readsAndOrders } from '../lib/mix.js';
import { loadShop } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

const TOTAL = num('TOTAL', 100000);
const RATE = num('RATE', 150);
const MIX = num('MIX', 0.95);
const MAX = env('PROFILE', 'default') === 'max';

function exactly() {
  const seconds = Math.ceil(TOTAL / RATE);
  const reads = Math.round(TOTAL * MIX);
  const orders = TOTAL - reads;
  // rate per timeUnit = the whole run: `rate` iterations, evenly spaced.
  const scenario = (count) => ({
    executor: 'constant-arrival-rate',
    rate: count,
    timeUnit: `${seconds}s`,
    duration: `${seconds}s`,
  });
  return readsAndOrders(RATE, MIX, scenario(reads), scenario(orders));
}

export const options = {
  scenarios: MAX
    ? mixedRampingScenarios({
        startRate: 100,
        stages: [{ target: num('MAX_RATE', 5000), duration: env('RAMP', '15m') }],
        mix: MIX,
      })
    : exactly(),
  thresholds: thresholds({ abort: MAX }),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export { reads, orders } from '../lib/mix.js';
