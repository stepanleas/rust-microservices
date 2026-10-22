services := catalog order customer restaurant payment api-gateway
rust_services := catalog customer order restaurant payment api-gateway
compose_dev := docker compose -f docker-compose/development/docker-compose.yml

build_images:
	@start=$$(date +%s); \
	for service in $(services); do \
		( \
			echo "==== Building $$service ===="; \
			docker build -f $$service/Dockerfile -t artellas/rust-$$service:$(version) .; \
		) & \
	done; \
	wait; \
	end=$$(date +%s); \
	echo "✅ Concurrent build took $$((end - start)) seconds."

build_dev_images:
	@start=$$(date +%s); \
	for service in $(services); do \
		( \
			echo "==== Building $$service ===="; \
			cd $$service && docker build -f Dockerfile-dev -t artellas/rust-$$service:dev . && cd - > /dev/null; \
		) & \
	done; \
	wait; \
	end=$$(date +%s); \
	echo "✅ Concurrent build took $$((end - start)) seconds."

remove_images:
	@for service in $(services); do \
		( \
			echo "==== Removing artellas/rust-$$service:$(version) ===="; \
			docker rmi artellas/rust-$$service:$(version); \
		) & \
	done; \
	wait

push_images:
	@for service in $(services); do \
		( \
			echo "==== Pushing artellas/rust-$$service:$(version) ===="; \
			docker push artellas/rust-$$service:$(version); \
		) & \
	done; \
	wait

# Infrastructure only (Postgres x5 + Kafka + topics), for running the services with `cargo run`.
infra_up:
	$(compose_dev) up -d catalog-db order-db customer-db restaurant-db payment-db kafka kafka-init

up:
	$(compose_dev) up -d --build catalog order customer restaurant payment api-gateway

down:
	$(compose_dev) down

# Unit + integration tests (testcontainers, needs Docker) of every service.
test_all:
	@for service in $(rust_services); do \
		echo "==== Testing $$service ===="; \
		(cd $$service && cargo test --workspace) || exit 1; \
	done

# End-to-end saga check against running services.
e2e:
	./scripts/e2e-saga.sh

# Production stack on images built from this checkout (`make build_images version=local`),
# with 3 replicas of order and restaurant.
compose_scaled := docker compose -f docker-compose/production/docker-compose.yml -f docker-compose/production/docker-compose.scale.yml
replicas ?= 3

scale_up:
	$(compose_scaled) up -d --wait --scale order=$(replicas) --scale restaurant=$(replicas)

scale_down:
	$(compose_scaled) down

# The saga check through the gateway, against the scaled stack.
e2e_scaled:
	ORDER_URL=http://localhost:8070/shop/order \
	RESTAURANT_URL=http://localhost:8070/shop/restaurant \
	PAYMENT_STOP_CMD="$(compose_scaled) stop payment" \
	PAYMENT_START_CMD="$(compose_scaled) start payment" \
	./scripts/e2e-saga.sh

# --- Kubernetes on kind (deploy/) -------------------------------------------------------------
# make kind_tool kind_up kind_images kind_deploy kind_e2e

# Downloads the kind version pinned in deploy/platform/versions.yaml to .tools/bin.
kind_tool:
	./deploy/scripts/kind-tool.sh

# Cluster (1 control plane + 2 workers) + platform charts (operators, observability stack: Grafana
# at http://localhost:8088/grafana once the shop is deployed).
kind_up:
	./deploy/scripts/kind-up.sh

# Builds the 6 images (tag local) and loads them into kind, restarting the deployed ones;
# SERVICES="order api-gateway" only those, SKIP_BUILD=1 only loads.
kind_images:
	./deploy/scripts/kind-load.sh

kind_deploy:
	./deploy/scripts/kind-deploy.sh

kind_e2e:
	BASE_URL=http://localhost:$${KIND_HTTP_PORT:-8088} TARGET=k8s KAFKA_HOST=localhost:$${KIND_KAFKA_PORT:-9093} \
		./scripts/e2e-saga.sh

kind_down:
	./deploy/scripts/kind-down.sh

# Fixed replicas for every service (KEDA paused), e.g. comparable load-test runs, and back to
# autoscaling: make pin_replicas N=3 [TARGET=lab]; make unpin_replicas [TARGET=lab]
pin_replicas:
	@test -n "$(N)" || { echo "usage: make pin_replicas N=<count> [TARGET=kind|lab]"; exit 1; }
	TARGET=$(TARGET) ./deploy/scripts/replicas.sh pin $(N)

unpin_replicas:
	TARGET=$(TARGET) ./deploy/scripts/replicas.sh unpin


# --- Load tests: k6 (load-tests/) ----------------------------------------------------------------
# make load_tool load_seed; make load SCENARIO=mixed; make load_verify     TARGET=kind (default) | lab

# Downloads the k6 version pinned in deploy/platform/versions.yaml to .tools/bin.
load_tool:
	./load-tests/scripts/k6-tool.sh

# Customers and products with a huge stock, once per cluster.
load_seed:
	SCENARIO=seed TARGET=$(TARGET) ./load-tests/scripts/run.sh

# k6 on this machine: make load SCENARIO=smoke|read-heavy|place-orders|mixed|stress|spike|soak|one-million
load:
	SCENARIO=$(SCENARIO) TARGET=$(TARGET) ./load-tests/scripts/run.sh

# k6 inside the cluster (k6-operator), runners on the control plane: make load_k8s SCENARIO=one-million PARALLELISM=4
load_k8s:
	SCENARIO=$(SCENARIO) PARALLELISM=$(PARALLELISM) TARGET=$(TARGET) ./load-tests/scripts/run-k8s.sh

# After a run: no stuck saga, no FAILED outbox row, empty DLQs, consistent restaurant stock.
load_verify:
	TARGET=$(TARGET) ./load-tests/scripts/verify.sh


# --- Homelab: Vagrant VMs + Ansible + kubeadm (infra/) ----------------------------------------
# Once: make lab_tools lab_host     Then: make lab_up lab_verify     See infra/README.md.
lab_ansible := ./infra/scripts/ansible.sh
lab_vagrant := ./infra/scripts/vagrant.sh

# Ansible + Python libraries in .tools/ansible (no sudo).
lab_tools:
	./infra/scripts/lab-tools.sh

# This machine: libvirt, Vagrant + vagrant-libvirt, lab network, split DNS, lab CA trust (sudo).
lab_host:
	$(lab_ansible) ansible-playbook playbooks/host.yml --ask-become-pass

# Creates the 4 VMs, then configures everything (ops services, kubeadm cluster, Cilium).
lab_up:
	$(lab_vagrant) up
	$(MAKE) lab_provision

# Re-applies the configuration; a second run reports changed=0.
lab_provision:
	$(lab_ansible) ansible-playbook site.yml

# Phase 3 checks; IDEMPOTENCY=1 also re-runs the provisioning and expects no changes.
lab_verify:
	./infra/scripts/lab-verify.sh

# Argo CD + the root Application (also part of lab_provision). REVISION=<branch> makes the lab
# follow a branch instead of main, e.g. to try it before merging.
lab_bootstrap:
	$(lab_ansible) ansible-playbook playbooks/argocd.yml $(if $(REVISION),-e argocd_revision=$(REVISION))

# A SealedSecret for the lab, safe to commit: make seal NAME=… NS=… FROM=<KEY=value file> [OUT=…]
seal:
	NAME=$(NAME) NS=$(NS) FROM=$(FROM) OUT=$(OUT) ./deploy/scripts/seal.sh

lab_status:
	@$(lab_vagrant) status
	@kubectl --kubeconfig $$HOME/.kube/lab.yaml get nodes -o wide 2>/dev/null || echo "cluster unreachable"

lab_ssh:
	$(lab_vagrant) ssh $(VM)

# Pause/resume the CI runner on the ops VM, e.g. around load tests.
# The marker file tells the LabCiRunnerOffline alert that the pause is on purpose.
lab_quiet:
	$(lab_ansible) ansible ops_vm -b -m ansible.builtin.shell -a "docker stop forgejo-runner && touch /opt/lab/node-exporter/ci-runner-paused"

lab_unquiet:
	$(lab_ansible) ansible ops_vm -b -m ansible.builtin.shell -a "docker start forgejo-runner && rm -f /opt/lab/node-exporter/ci-runner-paused"

lab_snapshot:
	$(lab_vagrant) snapshot save $(NAME)

lab_restore:
	$(lab_vagrant) snapshot restore $(NAME)

# Rolling upgrade, one minor version at a time: make lab_k8s_upgrade VERSION=1.37.1
lab_k8s_upgrade:
	@test -n "$(VERSION)" || { echo "usage: make lab_k8s_upgrade VERSION=x.y.z"; exit 1; }
	sed -i 's/^kubernetes_version: .*/kubernetes_version: $(VERSION)/' infra/ansible/inventory/group_vars/all/main.yml
	$(lab_ansible) ansible-playbook playbooks/upgrade-kubernetes.yml

# Rebuilds the cluster on the existing VMs (workloads and volumes are lost).
lab_k8s_reset:
	$(lab_ansible) ansible-playbook playbooks/reset.yml
	$(lab_ansible) ansible-playbook playbooks/kubernetes.yml

lab_renew_certs:
	$(lab_ansible) ansible-playbook playbooks/renew-certs.yml

lab_etcd_backup:
	$(lab_ansible) ansible-playbook playbooks/backup.yml --tags etcd

lab_backup:
	$(lab_ansible) ansible-playbook playbooks/backup.yml

# Boots the halted lab in stages: ops, then the cluster; the CI runner waits until it is up.
lab_start:
	./infra/scripts/lab-start.sh

lab_halt:
	$(lab_vagrant) halt

lab_destroy:
	$(lab_vagrant) destroy --force
