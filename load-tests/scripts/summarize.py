#!/usr/bin/env python3
"""One row of the runs table in load-tests/README.md, from a report directory of run.sh:

    summarize.py load-tests/reports/<run> [PROMETHEUS_URL]

With PROMETHEUS_URL (the run's Prometheus, reachable from here) it adds the largest Kafka
consumer lag seen during the run. Also writes the row to <run>/row.md.
"""
import json
import pathlib
import sys
import urllib.parse
import urllib.request


def read(path, default=""):
    try:
        return path.read_text().strip()
    except FileNotFoundError:
        return default


def ms(value):
    return "–" if value is None else (f"{value / 1000:.2f} s" if value >= 1000 else f"{value:.0f} ms")


def max_lag(prometheus, start, end):
    if not prometheus or not start or not end:
        return None
    seconds = max(60, int(float(end) - float(start)))
    query = f'max_over_time(sum(clamp_min(kafka_consumergroup_lag{{namespace="shop"}}, 0))[{seconds}s:15s])'
    url = f"{prometheus.rstrip('/')}/api/v1/query?" + urllib.parse.urlencode({"query": query, "time": end})
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            result = json.load(response)["data"]["result"]
        return int(float(result[0]["value"][1])) if result else 0
    except Exception as error:  # noqa: BLE001 - the row is still useful without it
        print(f"(no consumer lag: {error})", file=sys.stderr)
        return None


def main():
    report = pathlib.Path(sys.argv[1])
    prometheus = sys.argv[2] if len(sys.argv) > 2 else ""
    metrics = json.loads((report / "summary.json").read_text())["metrics"]
    env = dict(line.split("=", 1) for line in read(report / "env").splitlines() if "=" in line)
    name = report.name  # <yyyymmdd>-<hhmmss>-<scenario>-<target>
    date, _, scenario_target = name.split("-", 2)
    scenario, target = scenario_target.rsplit("-", 1)

    duration = metrics.get("http_req_duration", {})
    requests = metrics.get("http_reqs", {})
    failed = metrics.get("http_req_failed", {}).get("value", 0)
    saga = metrics.get("saga_e2e_duration", {})
    crossed = [f"{metric} {expression}" for metric, values in metrics.items()
               for expression, broken in values.get("thresholds", {}).items() if broken]
    lag = max_lag(prometheus, read(report / "started"), read(report / "finished"))

    row = " | ".join([
        f"{date[:4]}-{date[4:6]}-{date[6:]}", read(report / "commit", "?"), target, scenario,
        read(report / "replicas", "?"), env.get("RATE", "default"),
        f"{requests.get('count', 0):,}", f"{requests.get('rate', 0):.0f}",
        ms(duration.get("med")), ms(duration.get("p(95)")), ms(duration.get("p(99)")),
        ms(saga.get("p(95)")), "–" if lag is None else f"{lag:,}", f"{failed:.2%}",
        "pass" if not crossed else "FAIL: " + ", ".join(crossed),
    ])
    row = f"| {row} |"
    (report / "row.md").write_text(row + "\n")
    print(row)


if __name__ == "__main__":
    main()
