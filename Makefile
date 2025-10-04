services := catalog

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
	echo "Concurrent build took $$((end - start)) seconds."

push_images:
	@for service in $(services); do \
		( \
			echo "==== Pushing artellas/rust-$$service:$(version) ===="; \
			docker push artellas/rust-$$service:$(version); \
		) & \
	done; \
	wait