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

# Cluster (1 control plane + 2 workers) + platform operators (metrics-server, CNPG, Strimzi, Envoy Gateway).
kind_up:
	./deploy/scripts/kind-up.sh

# Builds the 6 images (tag local) and loads them into kind; SKIP_BUILD=1 only loads.
kind_images:
	./deploy/scripts/kind-load.sh

kind_deploy:
	./deploy/scripts/kind-deploy.sh

kind_e2e:
	BASE_URL=http://localhost:$${KIND_HTTP_PORT:-8088} TARGET=k8s KAFKA_HOST=localhost:$${KIND_KAFKA_PORT:-9093} \
		./scripts/e2e-saga.sh

kind_down:
	./deploy/scripts/kind-down.sh


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

lab_status:
	@$(lab_vagrant) status
	@kubectl --kubeconfig $$HOME/.kube/lab.yaml get nodes -o wide 2>/dev/null || echo "cluster unreachable"

lab_ssh:
	$(lab_vagrant) ssh $(VM)

# Pause/resume the CI runner on the ops VM, e.g. around load tests.
lab_quiet:
	$(lab_ansible) ansible ops_vm -b -m ansible.builtin.command -a "docker stop forgejo-runner"

lab_unquiet:
	$(lab_ansible) ansible ops_vm -b -m ansible.builtin.command -a "docker start forgejo-runner"

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

lab_halt:
	$(lab_vagrant) halt

lab_destroy:
	$(lab_vagrant) destroy --force
