// The pass/fail criteria of a run (see load-tests/README.md). `abort` = stop the run as soon as one
// breaks, for the scenarios that look for the breaking point.
export function thresholds({ abort = false } = {}) {
  const t = (expression) => (abort ? { threshold: expression, abortOnFail: true, delayAbortEval: '30s' } : expression);
  return {
    http_req_failed: [t('rate<0.01')],
    'http_req_duration{kind:read}': [t('p(95)<500')],
    'http_req_duration{kind:order}': [t('p(95)<1000')],
    // The target rate was reached: k6 had a free VU for every iteration. When looking for the
    // breaking point, a VU that comes free a moment late isn't one: under 1 dropped per second.
    dropped_iterations: [abort ? t('rate<1') : 'count==0'],
    // A sampled saga that didn't finish within SAGA_TIMEOUT_S.
    saga_timeouts: ['count==0'],
  };
}

// The summary shows these percentiles for every trend.
export const summaryTrendStats = ['avg', 'min', 'med', 'p(90)', 'p(95)', 'p(99)', 'max', 'count'];
