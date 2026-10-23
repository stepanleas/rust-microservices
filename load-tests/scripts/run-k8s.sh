#!/usr/bin/env bash
# Runs a k6 scenario inside the cluster with the k6-operator: PARALLELISM (4) runners pinned to
# the control-plane node, against the api-gateway Service (BASE_URL to override).
#   SCENARIO=hundred-thousand PARALLELISM=4 TARGET=lab load-tests/scripts/run-k8s.sh
# The scenario's settings (RATE, TOTAL, MIX, ...) pass through the environment. The runners'
# logs are archived in load-tests/reports/<time>-<scenario>-<target>-k8s/.
. "$(dirname "$0")/lib.sh"

SCENARIO=${SCENARIO:-smoke}
PARALLELISM=${PARALLELISM:-4}
K6_NAMESPACE=${K6_NAMESPACE:-k6}
RUNNER_NODE=${RUNNER_NODE:-control-plane}
script="$LOAD_DIR/k6/scenarios/$SCENARIO.js"
[[ -f "$script" ]] || { echo "no scenario $SCENARIO (load-tests/k6/scenarios/)"; exit 1; }
[[ -x "$K6" ]] || { echo "k6 not found: run 'make load_tool'"; exit 1; }
kube=("${KUBE[@]:0:3}" -n "$K6_NAMESPACE")

NAME="$SCENARIO-$(date +%H%M%S)"
K6_IMAGE_TAG=$(version k6.version)
K6_IMAGE_TAG=${K6_IMAGE_TAG#v}
export NAME K6_NAMESPACE PARALLELISM K6_IMAGE_TAG
# The runners write their metrics straight to the cluster's Prometheus (Shop / Live load test).
export ARGUMENTS="--tag testid=$NAME --tag target=$TARGET-k8s -o experimental-prometheus-rw"
if [[ "$RUNNER_NODE" == control-plane ]]; then
  export NODE_SELECTOR='{"node-role.kubernetes.io/control-plane": ""}'
else
  export NODE_SELECTOR='{}'
fi
# BASE_URL and every scenario setting set here reach the runners, and the archive: k6 computes
# the scenario's options (rates, durations) when it packs the script.
export BASE_URL=${BASE_URL:-http://api-gateway.$NAMESPACE:8070}
export K6_PROMETHEUS_RW_SERVER_URL=http://kube-prometheus-stack-prometheus.monitoring:9090/api/v1/write
export K6_FEATURES=native-histograms
export K6_SETUP_TIMEOUT=${K6_SETUP_TIMEOUT:-5m}
export K6_SYSTEM_TAGS=status,method,name,group,check,error_code,scenario,expected_response
settings=$(env | grep -E '^(K6_PROMETHEUS_RW_SERVER_URL|K6_FEATURES|K6_SETUP_TIMEOUT|K6_SYSTEM_TAGS|BASE_URL|DIRECT|RATE|TOTAL|MIX|DURATION|MAX_RATE|PEAK_RATE|BASE_RATE|HOLD|RAMP|PROFILE|STEPS|STEP_DURATION_S|SAGA_SAMPLE|SAGA_TIMEOUT_S|CUSTOMERS|VUS|MAX_VUS)=' || true)
archive_env=()
while IFS= read -r setting; do
  [[ -n "$setting" ]] && archive_env+=(-e "$setting")
done <<<"$settings"
RUNNER_ENV=$(python3 -c 'import sys, json; print(json.dumps([dict(zip(("name", "value"), l.split("=", 1))) for l in sys.argv[1].splitlines() if l]))' "$settings")
export RUNNER_ENV

finish() {
  "${kube[@]}" delete testrun "$NAME" --ignore-not-found >/dev/null || true
  "${kube[@]}" delete configmap "$NAME" --ignore-not-found >/dev/null || true
  if [[ "$TARGET" == lab && "${QUIET:-1}" == 1 ]]; then
    step "Resuming the CI runner"
    make -C "$ROOT_DIR" --no-print-directory lab_unquiet >/dev/null
  fi
}
trap finish EXIT
if [[ "$TARGET" == lab && "${QUIET:-1}" == 1 ]]; then
  step "Pausing the CI runner for the run"
  make -C "$ROOT_DIR" --no-print-directory lab_quiet >/dev/null
fi

report="$LOAD_DIR/reports/$(date +%Y%m%d-%H%M%S)-$SCENARIO-$TARGET-k8s"
mkdir -p "$report"
step "Packing $SCENARIO into the ConfigMap $K6_NAMESPACE/$NAME"
"$K6" archive "${archive_env[@]}" -O "$report/archive.tar" "$script"
"${kube[@]:0:3}" create namespace "$K6_NAMESPACE" --dry-run=client -o yaml | "${kube[@]:0:3}" apply -f - >/dev/null
"${kube[@]}" create configmap "$NAME" --from-file=archive.tar="$report/archive.tar"

step "Starting $PARALLELISM runners against $BASE_URL"
envsubst < "$LOAD_DIR/k6-operator/testrun.yaml" | "${kube[@]}" apply -f -

# Stages: initialization -> initialized -> created -> started -> stopped -> finished (or error).
stage=""
while [[ "$stage" != finished && "$stage" != error ]]; do
  sleep 10
  new=$("${kube[@]}" get testrun "$NAME" -o jsonpath='{.status.stage}')
  [[ "$new" != "$stage" ]] && echo "  $(date +%T) $new"
  stage=$new
done
# The runners' logs (the end-of-test summary, threshold results); the TestRun goes on exit.
for pod in $("${kube[@]}" get pods -l "k6_cr=$NAME,runner=true" -o name); do
  "${kube[@]}" logs "$pod" > "$report/${pod#pod/}.log" 2>&1 || true
done

# A runner whose share of the run broke a threshold says so in its log.
failed=0
for log in "$report"/*.log; do
  [[ -f "$log" ]] || continue
  grep -qE "thresholds on metrics .* have been crossed|level=error" "$log" && failed=1
done
git -C "$ROOT_DIR" rev-parse --short HEAD > "$report/commit" 2>/dev/null || true
if [[ "$stage" == error || "$failed" == 1 ]]; then
  echo "The run failed (stage $stage); see ${report#"$ROOT_DIR"/}/*.log"
  exit 1
fi
echo "Done; the runners' logs are in ${report#"$ROOT_DIR"/}"
