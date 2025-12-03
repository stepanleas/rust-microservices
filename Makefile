services := catalog order customer

build_images:
	@start=$$(date +%s); \
	for service in $(services); do \
		( \
			echo "==== Building $$service ===="; \
			cd $$service && docker build -t artellas/rust-$$service:$(version) . && cd - > /dev/null; \
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
	( \
		echo "==== Removing artellas/rust-spring-boot-api-gateway:$(version) ===="; \
		docker rmi artellas/rust-spring-boot-api-gateway:$(version); \
	) & \
	wait

push_images:
	@for service in $(services); do \
		( \
			echo "==== Pushing artellas/rust-$$service:$(version) ===="; \
			docker push artellas/rust-$$service:$(version); \
		) & \
	done; \
	( \
		echo "==== Pushing artellas/rust-spring-boot-api-gateway:$(version) ===="; \
		docker push artellas/rust-spring-boot-api-gateway:$(version); \
	) & \
	wait