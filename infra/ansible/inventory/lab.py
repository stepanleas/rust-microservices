#!/usr/bin/env python3
"""Ansible inventory rendered from ../../lab.yml, so the topology lives in one file.

Groups: ops, control_plane, workers, kubernetes (control_plane + workers), and lab_host for the
machine running Ansible. Replace this script with a static inventory to target real hardware.
"""
import json
import pathlib
import sys

import yaml

INFRA = pathlib.Path(__file__).resolve().parents[2]


def inventory():
    lab = yaml.safe_load((INFRA / "lab.yml").read_text())
    groups = {"all": {"children": ["lab_host", "lab"]}, "lab": {"children": []}}
    hostvars = {
        "localhost": {
            "ansible_connection": "local",
            "ansible_python_interpreter": sys.executable,
        }
    }
    groups["lab_host"] = {"hosts": ["localhost"]}
    for vm in lab["vms"]:
        name = vm["name"]
        hostvars[name] = {
            "ansible_host": vm["ip"],
            "ansible_user": "vagrant",
            "ansible_ssh_private_key_file": str(
                INFRA / ".vagrant" / "machines" / name / "libvirt" / "private_key"
            ),
            "lab_ip": vm["ip"],
        }
        for group in vm["groups"]:
            groups.setdefault(group, {"hosts": []})["hosts"].append(name)
            if group not in groups["lab"]["children"] and group != "kubernetes":
                groups["lab"]["children"].append(group)
    groups["kubernetes"] = {"children": ["control_plane", "workers"]}
    groups["_meta"] = {"hostvars": hostvars}
    groups["all"]["vars"] = {"lab": lab}
    return groups


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--host":
        print("{}")
    else:
        print(json.dumps(inventory(), indent=2))
