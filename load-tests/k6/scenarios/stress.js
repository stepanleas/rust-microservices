// Finds the knee: the mixed traffic ramps from 50 requests/s to MAX_RATE (3000) in STEPS (6) equal
// steps of STEP_DURATION (2m) each, and the run stops as soon as a threshold breaks (the failure
// rate, a p95, or k6 running out of VUs). The rate reached before that is the system's capacity.
import { num } from '../lib/config.js';
import { mixedRampingScenarios } from '../lib/mix.js';
import { loadShop } from '../lib/shop.js';
import { summaryTrendStats, thresholds } from '../lib/thresholds.js';

const MAX_RATE = num('MAX_RATE', 3000);
const STEPS = num('STEPS', 6);
const STEP = `${num('STEP_DURATION_S', 120)}s`;

const stages = [];
for (let i = 1; i <= STEPS; i++) {
  const target = Math.round((MAX_RATE * i) / STEPS);
  stages.push({ target, duration: '30s' }, { target, duration: STEP });
}

export const options = {
  scenarios: mixedRampingScenarios({ startRate: 50, stages, mix: num('MIX', 0.8) }),
  thresholds: thresholds({ abort: true }),
  summaryTrendStats,
};

export const setup = () => loadShop({ orders: 20 });

export { reads, orders } from '../lib/mix.js';
