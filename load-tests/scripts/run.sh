#!/usr/bin/env bash
# Runs a k6 scenario from this machine against kind or the lab:
#   SCENARIO=smoke|read-heavy|place-orders|mixed|stress|spike|soak|one-million|seed  (default smoke)
#   TARGET=kind (default, http://localhost:8088) | lab (https://shop.lab.internal)
# The scenarios' own settings (RATE, DURATION, MIX, ...) pass through the environment. On the lab
# the CI runner pauses for the run (make lab_quiet / lab_unquiet).
#
# While it runs: k6 writes its metrics to the cluster's Prometheus (Grafana: Shop / Live load test,
# PROM=0 to skip) and serves its own web dashboard on http://localhost:5665. Afterwards
# load-tests/reports/<time>-<scenario>-<target>/ holds the summary, the web dashboard's HTML report
# and row.md, the run's line for the table in load-tests/README.md.
. "$(dirname "$0")/lib.sh"

SCENARIO=${SCENARIO:-smoke}
[[ -x "$K6" ]] || { echo "k6 not found: run 'make load_tool'"; exit 1; }

if [[ "$SCENARIO" == seed ]]; then
  script="$LOAD_DIR/k6/seed.js"
else
  script="$LOAD_DIR/k6/scenarios/$SCENARIO.js"
fi
[[ -f "$script" ]] || { echo "no scenario $SCENARIO (load-tests/k6/scenarios/)"; exit 1; }

forward="" quiet=""
cleanup() {
  [[ -n "$forward" ]] && kill "$forward" 2>/dev/null
  if [[ -n "$quiet" ]]; then
    step "Resuming the CI runner"
    make -C "$ROOT_DIR" --no-print-directory lab_unquiet >/dev/null
  fi
}
trap cleanup EXIT

case "$TARGET" in
  kind) export BASE_URL=${BASE_URL:-http://localhost:$KIND_HTTP_PORT} ;;
  lab)
    export BASE_URL=${BASE_URL:-https://shop.lab.internal}
    # k6 trusts the lab CA.
    export SSL_CERT_FILE=${SSL_CERT_FILE:-$HOME/.lab/ca/ca.crt}
    if [[ "$SCENARIO" != seed && "${QUIET:-1}" == 1 ]]; then
      step "Pausing the CI runner for the run"
      make -C "$ROOT_DIR" --no-print-directory lab_quiet >/dev/null
      quiet=1
    fi
    ;;
esac

report="$LOAD_DIR/reports/$(date +%Y%m%d-%H%M%S)-$SCENARIO-$TARGET"
mkdir -p "$report"
outputs=()
prometheus=""
if [[ "${PROM:-1}" == 1 ]]; then
  if [[ "$TARGET" == kind ]]; then
    # kind doesn't expose Prometheus: forward a local port for the run.
    port=${PROM_PORT:-19090}
    "${KUBE[@]:0:3}" -n monitoring port-forward svc/kube-prometheus-stack-prometheus "$port:9090" >/dev/null 2>&1 &
    forward=$!
    prometheus=http://127.0.0.1:$port
    for _ in $(seq 1 20); do curl -fsS "$prometheus/-/ready" >/dev/null 2>&1 && break; sleep 0.5; done
  else
    prometheus=https://prometheus.lab.internal
  fi
  export K6_PROMETHEUS_RW_SERVER_URL=$prometheus/api/v1/write
  outputs+=(-o experimental-prometheus-rw)
fi
# Native histograms: Grafana computes any percentile over any window, across runners.
export K6_FEATURES=native-histograms
# setup() waits up to 60 s per new customer until the order service knows it (longer while the
# consumers rebalance, e.g. right after pin_replicas); k6's default allows 60 s for all of it.
export K6_SETUP_TIMEOUT=${K6_SETUP_TIMEOUT:-5m}
# No `url` tag (one series per id) nor connection details: these become Prometheus labels.
export K6_SYSTEM_TAGS=${K6_SYSTEM_TAGS:-status,method,name,group,check,error_code,scenario,expected_response}
export K6_WEB_DASHBOARD=${K6_WEB_DASHBOARD:-true} K6_WEB_DASHBOARD_OPEN=false
export K6_WEB_DASHBOARD_EXPORT="$report/report.html"

step "k6: $SCENARIO against $BASE_URL (report: ${report#"$ROOT_DIR"/})"
"${KUBE[@]}" get deployments -l app.kubernetes.io/part-of=rust-microservices \
  -o jsonpath='{range .items[*]}{.spec.replicas}{"\n"}{end}' 2>/dev/null | sort -u | paste -sd/ > "$report/replicas" || true
git -C "$ROOT_DIR" rev-parse --short HEAD > "$report/commit" 2>/dev/null || true
env | grep -E '^(RATE|TOTAL|MIX|DURATION|MAX_RATE|PEAK_RATE|PROFILE|DIRECT|SAGA_SAMPLE)=' | sort > "$report/env" || true
date +%s > "$report/started"
status=0
"$K6" run --tag "testid=$(basename "$report")" --tag "target=$TARGET" "${outputs[@]}" \
  --summary-export "$report/summary.json" "$script" "$@" || status=$?
date +%s > "$report/finished"
if [[ -f "$report/summary.json" && "$SCENARIO" != seed ]]; then
  step "Result (add it to the runs table in load-tests/README.md)"
  "$LOAD_DIR/scripts/summarize.py" "$report" "$prometheus" || true
fi
exit "$status"
