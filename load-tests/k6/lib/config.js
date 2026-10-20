// Where and how hard the scenarios run, from the environment (`k6 run -e NAME=value` or
// `NAME=value make load ...`):
//   BASE_URL   http://localhost:8088 (kind, the default), https://shop.lab.internal (lab),
//              http://api-gateway.shop:8070 (k6-operator runs inside the cluster)
//   DIRECT=1   /direct/<service>/... instead of /shop/<service>/...: the services without the
//              api-gateway hop (Envoy routes them on kind and the lab)
//   RATE, DURATION, MIX, ...   per scenario, see its header
export const BASE_URL = env('BASE_URL', 'http://localhost:8088').replace(/\/+$/, '');

const PREFIX = env('DIRECT', '') === '1' ? 'direct' : 'shop';

export function url(service, path) {
  return `${BASE_URL}/${PREFIX}/${service}${path}`;
}

export function env(name, fallback) {
  const value = __ENV[name];
  return value === undefined || value === '' ? fallback : value;
}

export function num(name, fallback) {
  const value = Number(env(name, fallback));
  if (Number.isNaN(value)) {
    throw new Error(`${name} must be a number, got ${__ENV[name]}`);
  }
  return value;
}

// Pre-allocated and maximum VUs for an arrival rate (iterations/s): enough for ~100 ms responses,
// with room for slower ones. Orders also keep a VU busy while it follows a sampled saga, up to
// SAGA_TIMEOUT_S each. VUS / MAX_VUS override.
export function vusFor(rate, { orders = false } = {}) {
  const polling = orders ? Math.ceil(rate * num('SAGA_SAMPLE', 0.01) * num('SAGA_TIMEOUT_S', 60)) : 0;
  return {
    preAllocatedVUs: num('VUS', Math.max(10, Math.ceil(rate / 10))),
    maxVUs: num('MAX_VUS', Math.max(50, Math.ceil(rate)) + polling),
  };
}
