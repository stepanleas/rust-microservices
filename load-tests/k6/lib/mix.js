// Reads and orders as two scenarios of one run, so the mix is exact rather than random: `reads`
// gets the MIX share of the rate, `orders` the rest.
import { vusFor } from './config.js';
import { placeOrder, readOnce } from './shop.js';

export function reads(shop) {
  readOnce(shop);
}

export function orders(shop) {
  placeOrder(shop);
}

// Two scenarios from their executor settings; VUs sized for each share of `rate`.
export function readsAndOrders(rate, mix, readScenario, orderScenario) {
  return {
    reads: { ...vusFor(rate * mix), ...readScenario, exec: 'reads' },
    orders: { ...vusFor(rate * (1 - mix), { orders: true }), ...orderScenario, exec: 'orders' },
  };
}

export function mixedScenarios({ rate, duration, mix }) {
  const constant = (share) => ({ executor: 'constant-arrival-rate', rate: Math.max(1, Math.round(rate * share)), timeUnit: '1s', duration });
  return readsAndOrders(rate, mix, constant(mix), constant(1 - mix));
}

// `stages` targets are total requests/s; each scenario gets its share.
export function mixedRampingScenarios({ startRate, stages, mix }) {
  const peak = Math.max(startRate, ...stages.map((s) => s.target));
  const ramping = (share) => ({
    executor: 'ramping-arrival-rate',
    startRate: Math.max(1, Math.round(startRate * share)),
    timeUnit: '1s',
    stages: stages.map((s) => ({ target: Math.max(1, Math.round(s.target * share)), duration: s.duration })),
  });
  return readsAndOrders(peak, mix, ramping(mix), ramping(1 - mix));
}
