services := catalog order customer restaurant payment api-gateway
rust_services := catalog customer order restaurant payment api-gateway
compose_dev := docker compose -f docker-compose/development/docker-compose.yml

build_images:
	@start=$$(date +%s); \
	for service in $(services); do \
		( \
			echo "==== Building $$service ===="; \
			cd $$service && docker build -t artellas/rust-$$service:$(version) . && cd - > /dev/null; \
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

