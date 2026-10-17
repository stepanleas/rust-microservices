import http from 'k6/http';
import { check } from 'k6';

import { url } from './config.js';

// Every request carries a `kind` tag, which the thresholds and dashboards select on:
//   read       GETs of the read path
//   order      POST /api/orders (starts a saga)
//   saga-poll  GET of a sampled order until its saga finished (not part of the request budget)
//   seed       test data setup
// `name` is the route template, so URLs with ids don't create a time series each. `probe`: a 4xx
// is an expected answer (e.g. an order for a customer the order service doesn't know yet), not a
// failed request.
const PROBE = http.expectedStatuses({ min: 200, max: 499 });

export function get(service, path, kind, name, { probe = false } = {}) {
  const params = { tags: { kind, name: `GET ${service} ${name}` } };
  return checked(http.get(url(service, path), probe ? { ...params, responseCallback: PROBE } : params), params, probe);
}

export function post(service, path, body, kind, name, { probe = false } = {}) {
  const params = { headers: { 'Content-Type': 'application/json' }, tags: { kind, name: `POST ${service} ${name}` } };
  return checked(
    http.post(url(service, path), JSON.stringify(body), probe ? { ...params, responseCallback: PROBE } : params),
    params,
    probe,
  );
}

// The `data` of a JSON response, or undefined.
export function data(response) {
  try {
    return response.json('data');
  } catch (_) {
    return undefined;
  }
}

function checked(response, params, probe) {
  if (!probe) {
    check(response, { [`${params.tags.name} is 2xx`]: (r) => r.status >= 200 && r.status < 300 });
  }
  return response;
}
