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

build_dev_images:
	@start=$$(date +%s); \
	for service in $(services); do \
		( \
			echo "==== Building $$service ===="; \
			cd $$service && docker build -f Dockerfile-dev -t artellas/rust-$$service:dev . && cd - > /dev/null; \
		) & \
	done; \
	( \
		echo "==== Building gateway ===="; \
		cd spring-boot-api-gateway && mvn clean compile jib:dockerBuild && cd - > /dev/null; \
	) & \
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

