#!/usr/bin/env bash
# Runs a CI workflow (or one job of it) on this machine with the lab's runner and job image, to
# try a workflow change without pushing. The jobs get the lab's variables, not its secrets.
# Usage: WORKFLOW=infra.yml [JOB=kubernetes] ci-exec.sh
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
# The runner and job images the lab uses.
setting() { sed -n "s/^$1: //p" "$ROOT/infra/ansible/inventory/group_vars/all/main.yml"; }

: "${WORKFLOW:?usage: make ci_exec WORKFLOW=infra.yml [JOB=kubernetes]}"

# Root in the runner container (the Docker socket), with git trusting the repository owned by
# this user; the job containers get a copy of the workspace.
exec docker run --rm -u 0 \
  -v /var/run/docker.sock:/var/run/docker.sock -v "$ROOT:$ROOT" -w "$ROOT" \
  -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory -e GIT_CONFIG_VALUE_0='*' \
  "$(setting forgejo_runner_image)" \
  forgejo-runner exec -W ".github/workflows/$WORKFLOW" ${JOB:+-j "$JOB"} \
  --image "$(setting forgejo_runner_job_image)" \
  --default-actions-url https://github.com \
  --var CI_PLATFORM=lab
