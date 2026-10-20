#!/usr/bin/env bash
# Checks the homelab against Phase 3's "done when" list: nodes, Cilium, trusted HTTPS on the ops
# services, docker.io pulls through Zot, and a LoadBalancer IP reachable from this machine.
# IDEMPOTENCY=1 also re-runs site.yml and expects changed=0.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
DOMAIN=$(python3 -c "import yaml; print(yaml.safe_load(open('$ROOT/infra/lab.yml'))['domain'])")
export KUBECONFIG=${KUBECONFIG_LAB:-$HOME/.kube/lab.yaml}
NS=lab-verify
failures=0

check() {
  local description=$1; shift
  local output
  if output=$("$@" 2>&1); then
    printf '  \033[32mok\033[0m    %s\n' "$description"
  else
    printf '  \033[31mFAIL\033[0m  %s\n%s\n' "$description" "$(sed 's/^/          /' <<<"$output" | tail -5)"
    failures=$((failures + 1))
  fi
}

nodes_ready() {
  local ready
  ready=$(kubectl get nodes --no-headers | awk '$2 == "Ready"' | wc -l)
  kubectl get nodes --no-headers
  [[ $ready -eq 3 ]]
}

cilium_ok() {
  kubectl -n kube-system exec ds/cilium -c cilium-agent -- cilium-dbg status --brief
}

https_ok() { curl --fail --silent --show-error --output /dev/null "$1"; }

# Anonymous pulls are allowed, so /v2/ answers 200 over a certificate curl trusts.
registry_ok() { https_ok "https://registry.$DOMAIN/v2/"; }

pull_through_zot() {
  kubectl -n "$NS" run busybox --image=docker.io/library/busybox:1.37 --restart=Never \
    --command -- true >/dev/null
  kubectl -n "$NS" wait --for=jsonpath='{.status.phase}'=Succeeded pod/busybox --timeout=3m
  curl --fail --silent "https://mirror.$DOMAIN/v2/_catalog" | grep -q '"library/busybox"'
}

load_balancer_ok() {
  kubectl -n "$NS" create deployment web --image=docker.io/library/busybox:1.37 -- \
    sh -c 'mkdir /www && echo lab-ok > /www/index.html && exec httpd -f -p 8080 -h /www' >/dev/null
  kubectl -n "$NS" expose deployment web --type=LoadBalancer --port=80 --target-port=8080 >/dev/null
  kubectl -n "$NS" rollout status deployment/web --timeout=3m >/dev/null
  local ip=""
  for _ in $(seq 1 30); do
    ip=$(kubectl -n "$NS" get service web -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
    [[ -n $ip ]] && break
    sleep 2
  done
  echo "LoadBalancer IP: ${ip:-none}"
  [[ $ip == 192.168.56.2* ]] || return 1
  for _ in $(seq 1 15); do
    curl --silent --max-time 2 "http://$ip/" | grep -q lab-ok && return 0
    sleep 2
  done
  return 1
}

idempotent() {
  local recap
  recap=$("$ROOT/infra/scripts/ansible.sh" ansible-playbook site.yml </dev/null 2>&1 | cat | tee /dev/stderr | sed -n '/PLAY RECAP/,$p')
  echo "$recap"
  ! grep -E 'changed=[1-9]|failed=[1-9]|unreachable=[1-9]' <<<"$recap"
}

echo "Lab checks ($KUBECONFIG)"
kubectl delete namespace "$NS" --ignore-not-found --wait=true >/dev/null 2>&1
kubectl create namespace "$NS" >/dev/null

check "3 nodes Ready" nodes_ready
check "Cilium healthy" cilium_ok
check "https://git.$DOMAIN answers with a trusted certificate" https_ok "https://git.$DOMAIN/api/healthz"
check "https://registry.$DOMAIN/v2/ answers with a trusted certificate" registry_ok
check "docker.io/library/busybox is pulled through Zot" pull_through_zot
check "a LoadBalancer Service gets a 192.168.56.2xx IP reachable from this machine" load_balancer_ok
if [[ ${IDEMPOTENCY:-0} == 1 ]]; then
  check "a second provisioning run changes nothing" idempotent
fi

kubectl delete namespace "$NS" --wait=false >/dev/null 2>&1
if [[ $failures -gt 0 ]]; then
  echo "$failures check(s) failed"
  exit 1
fi
echo "All lab checks passed."
