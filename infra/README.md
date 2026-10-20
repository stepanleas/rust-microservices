# Homelab: Vagrant + Ansible + kubeadm

A prod-like environment on this machine, with no cloud: four KVM VMs on a private network.
- **ops** runs DNS, a CA, the container registry, the git forge with CI, and notifications.
- **k8s-cp, k8s-w1, k8s-w2** form a three-node kubeadm cluster with Cilium.

kind (`make kind_*`) stays the fast inner loop. The lab is where the system runs as it would on
real machines.

| VM | IP | vCPU / RAM / disk | Runs |
|---|---|---|---|
| `ops` | 192.168.56.10 | 4 / 6 GB / 80 GB | dnsmasq, Caddy (TLS), Zot registry + docker.io cache, Forgejo + Actions runner, ntfy, node-exporter, Alloy |
| `k8s-cp` | .11 | 2 / 4 GB / 30 GB | kubeadm control plane (tainted: no workloads) |
| `k8s-w1`, `k8s-w2` | .12, .13 | 4 / 6.5 GB / 60 GB | workloads |
| LoadBalancer pool | .200–.220 | | Cilium LB-IPAM, announced over ARP; `.200` is the cluster ingress |

That's 23 GB of RAM and 14 vCPUs. CPU is overcommitted on 8 host threads, which is fine for a
lab. Disks are thin-provisioned. **Stop kind first** (`make kind_down`, or `docker stop` its
node containers): both together don't fit in 31 GB.

## Who does what

- **Vagrant** creates empty VMs (CPU, RAM, disk, IP) from `lab.yml`. That's all it does.
- **Ansible** turns them, and this machine, into the lab. Its tasks are declarative and
  idempotent, so re-running only fixes drift, and a second run reports `changed=0`.
- **Argo CD** manages everything inside Kubernetes once the cluster exists (see
  [CI/CD and GitOps](#cicd-and-gitops)). Ansible only installs it and writes what can't be in git.

| Area | Ansible's job |
|---|---|
| This machine (sudo, `make lab_host`) | libvirt, Vagrant + vagrant-libvirt, the isolated `lab` network (bridge `virbr-lab`), split DNS for `lab.internal` through systemd-resolved, lab CA in the system store and in Chrome/Firefox |
| Every VM | apt baseline, chrony, swap off, kernel modules and sysctls, lab CA trusted, `*.lab.internal` resolved by the ops VM |
| ops | dnsmasq; Docker; Caddy with the lab wildcard certificate; Zot; Forgejo (admin user, org `shop`, repo `rust-microservices`, Actions on); runner registered offline with a shared secret; ntfy; node-exporter (plus the lab certificates' expiry and the CI runner's state as textfile metrics); Alloy shipping journald and container logs to Loki |
| Kubernetes nodes | containerd + runc from upstream releases, image pulls through Zot; kubelet/kubeadm/kubectl pinned and held; `kubeadm init` from a versioned config (no kube-proxy, metrics endpoints open, kubelet serving certificates); workers joined; Cilium; kubelet-csr-approver; LoadBalancer IP pool + L2 policy |
| Day 2 | rolling Kubernetes upgrades, certificate renewal, cluster reset, backups |

The secrets (Forgejo admin password, runner secret, registry CI password) are random. They're
created on first use in `ansible/inventory/group_vars/all/vault.yml`, encrypted with Ansible
Vault, and its password is in `~/.lab/vault-pass`. **Back up `~/.lab/`**: it also holds the
lab CA, and nothing can decrypt the vault without the password.

## First run

```bash
make lab_tools        # Ansible in .tools/ansible (uv-managed Python 3.13), no sudo
make lab_host         # once, in a terminal: asks for your sudo password (never stored)
make lab_up           # vagrant up (4 VMs) + ansible-playbook site.yml, ~20-30 min the first time
make lab_verify       # the checks below; IDEMPOTENCY=1 also re-runs the provisioning
```

`make lab_host` adds you to the `libvirt` group. Until you log in again, the scripts run
Vagrant through `sg libvirt`.

Then:
- `kubectl --context lab get nodes`: the kubeconfig is `~/.kube/lab.yaml`, merged into
  `~/.kube/config` (the previous file is kept as `~/.kube/config.before-lab`).
- https://git.lab.internal: Forgejo. The user is `lab-admin`; the password is in the vault:
  `infra/scripts/ansible.sh ansible-vault view inventory/group_vars/all/vault.yml`.
- https://registry.lab.internal: Zot. Pulls are anonymous; pushes need user `forgejo-ci`.
- https://argocd.lab.internal: Argo CD. The user is `admin`; the password is in
  `kubectl --context lab -n argocd get secret argocd-initial-admin-secret -o jsonpath='{.data.password}' | base64 -d`.
- https://shop.lab.internal: the shop, through the api-gateway (`/shop/<service>/...`).
- https://ntfy.lab.internal: ntfy.

`make lab_verify` checks:
- 3 nodes are Ready and Cilium is healthy;
- git and registry answer over HTTPS with a certificate this machine trusts;
- a pod's `docker.io/library/busybox` is pulled through the Zot mirror;
- a `LoadBalancer` Service gets a `192.168.56.2xx` address that this machine can reach.

## Daily use

| Target | |
|---|---|
| `make lab_status` | VM states and nodes |
| `make lab_provision` | re-apply the configuration (after editing `lab.yml`, versions or roles) |
| `make lab_ssh VM=k8s-w1` | shell on a VM |
| `make lab_quiet` / `lab_unquiet` | stop/start the CI runner, e.g. during load tests (`LabCiRunnerOffline` ignores such a pause for 3 h) |
| `make lab_snapshot NAME=x` / `lab_restore NAME=x` | VM snapshots before risky experiments |
| `make lab_k8s_upgrade VERSION=1.37.1` | bumps `kubernetes_version` and upgrades: control plane, then each worker (drain → upgrade → uncordon). One minor version at a time |
| `make lab_renew_certs` | renews kubeadm's certificates (they expire after a year; upgrades also renew them) |
| `make lab_k8s_reset` | rebuilds the cluster on the existing VMs (workloads are lost) |
| `make lab_etcd_backup` / `lab_backup` | etcd snapshot + PKI (and a Forgejo dump) to `~/.lab/backups/<time>/` |
| `make lab_halt` / `lab_destroy` | stop / delete the VMs (`~/.lab` is kept, so a rebuild reuses the CA) |
| `make lab_bootstrap [REVISION=<branch>]` | Argo CD + the root Application (also part of `lab_provision`); `REVISION` makes the lab follow a branch instead of `main` |
| `make seal NAME=… NS=… FROM=<file> [OUT=…]` | a SealedSecret from a `KEY=value` file, safe to commit; only the lab cluster can decrypt it |

## CI/CD and GitOps

The workflows in `.github/workflows` run unchanged on GitHub and on the lab's Forgejo (there is
no `.forgejo/workflows`, so Forgejo uses them). Everything that differs comes from repository
variables and secrets, which the `forgejo` role sets on the lab and which are unset on GitHub:

| | GitHub | Lab (Forgejo) |
|---|---|---|
| `ci.yml` | fmt, clippy, tests of the changed crates | the same, in job containers on the ops VM |
| `release.yml` on `<service>-v<semver>` | `artellas/rust-<service>` on Docker Hub, gha build cache | `registry.lab.internal/shop/rust-<service>`, registry build cache, then `deploy-lab` |
| `infra.yml` on `infra/**`, `deploy/**` | ansible-lint, helm lint, kubeconform | the same |
| Variables | none | `CI_PLATFORM=lab`, `REGISTRY_HOST`, `IMAGE_NAMESPACE=shop`, `LAB_ENV_REPO`, `BUILDKITD_CONFIG` |
| Secrets | `DOCKER_USERNAME/PASSWORD` (Docker Hub) | the Zot user, `LAB_DEPLOY_TOKEN` |

Flow on the lab: `git push lab order-v1.4.0` → release builds and pushes the image to Zot →
`deploy-lab` writes the tag to `images.yaml` in the repository `shop/lab-env` → Argo CD (polling
every 60 s) syncs `rust-microservices` → rolling update. The bot user `lab-deploy` can write only
to `lab-env`, so nothing ever commits to `main`, and `main` stays identical on GitHub and the lab.
The image tags live in a repository of their own because an Argo CD application can't combine
two revisions of the same repository.

```
deploy/argocd/                  chart rendered by the root Application (roles/argocd)
├── charts.yaml -> ../platform/charts.yaml
└── templates/
    ├── platform.yaml           one Application per platform chart (sync waves: storage,
    │                           operators, Argo CD itself), values from deploy/platform/<name>/
    ├── lab-config.yaml         deploy/platform/*/lab/*.yaml, e.g. the HTTPS Gateway on .200
    └── rust-microservices.yaml chart + values-lab.yaml + images.yaml from lab-env
```

`deploy/platform/charts.yaml` pins every platform chart for both kind (`kind-up.sh`) and the
lab. Ansible writes only what can't be in git: the wildcard certificate secret for the Gateway
and the lab CA for Argo CD (both from `~/.lab/ca`), and the root Application.

Git: `git push lab <branch>` / `git push lab <tag>`; `git push lab --all` re-pushes every branch
after the lab was down. Git reads the lab token from `~/.lab/forgejo-token` (credential helper in
`.git/config`). `infra/scripts/lab-remote.sh` also adds the lab as a second push URL of `origin`,
so a plain `git push` goes to both.

## Layout

```
infra/
├── lab.yml               topology: names, IPs, sizes, network, box (Vagrantfile + inventory)
├── Vagrantfile
├── scripts/              ansible.sh / vagrant.sh wrappers, lab-tools, lab-secrets, lab-verify,
│                         lab-remote
└── ansible/
    ├── ansible.cfg, .ansible-lint, requirements.yml
    ├── inventory/lab.py               groups ops_vm, control_plane, workers, kubernetes from lab.yml
    ├── inventory/group_vars/all/      main.yml (versions, settings), vault.yml (encrypted)
    ├── site.yml                       common → ops → kubernetes → argocd
    ├── playbooks/                     host, common, ops, kubernetes, argocd, upgrade-kubernetes,
    │                                  renew-certs, reset, backup
    └── roles/                         one per concern (lab_ca, common, dnsmasq, docker, caddy, zot,
                                       forgejo, forgejo_runner, ntfy, node_exporter, alloy, containerd,
                                       kube_packages, kubeadm_control_plane, kubeadm_worker,
                                       cilium, lab_kubeconfig, argocd, lab_host, lab_host_user)
```

Names: every `*.lab.internal` resolves to the ingress IP `.200`, except the ones served by
`ops` (`ops`, `git`, `registry`, `mirror`, `ntfy`) and the VM names. TLS: the lab CA in
`~/.lab/ca` is name-constrained to `lab.internal` and `192.168.56.0/24`, so trusting it in a
browser can't be abused for other sites. Caddy serves a one-year wildcard certificate, renewed
by `lab_provision` 30 days before it expires.

## Troubleshooting

- **`vagrant up` can't find the box or the provider:** run `make lab_host` again; check
  `vagrant plugin list` shows vagrant-libvirt.
- **`*.lab.internal` doesn't resolve on this machine:** `resolvectl status virbr-lab` should
  list `192.168.56.10` and `~lab.internal`. The libvirt hook sets that each time the network
  starts. `sudo /etc/libvirt/hooks/network lab started begin -` applies it by hand.
- **Image pulls are slow or fail in the cluster:** `curl https://mirror.lab.internal/v2/_catalog`.
  containerd falls back to Docker Hub when the mirror is down, so pulls still work, but they're
  rate-limited.
- **A node is NotReady after a host reboot:** `make lab_up` starts the VMs again (provisioning
  is idempotent). Kubernetes recovers by itself within a minute or two.
- **CI runs stay `waiting` on the lab:** the runner can stop taking jobs after it was restarted
  while jobs were running. `make lab_ssh VM=ops` then `sudo docker restart forgejo-runner`.
- **On the lab, the first push of a branch tests every crate:** without a previous commit,
  `scripts/ci/changed-crates.sh` compares against `origin/main`.
- **Start over:** `make lab_destroy lab_up`. The CA and secrets in `~/.lab` stay, so browsers
  keep trusting the lab.

## Moving to real hardware

Only the inventory knows about Vagrant. To run the lab on 2–3 mini PCs or Proxmox VMs:
1. replace `inventory/lab.py` with a static inventory (same groups, `lab_ip` per host);
2. run `site.yml`. The libvirt parts of `host.yml` are no longer needed, but the CA trust and
   split DNS still are.
