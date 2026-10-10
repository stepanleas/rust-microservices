DB_URL = postgres://postgres:postgres@localhost:5435/restaurant_db

migration:
	diesel migration generate $(name)

migrate_up:
	diesel migration run --database-url=$(DB_URL)

migrate_down:
	diesel migration revert --database-url=$(DB_URL)

print_schema:
	diesel print-schema --database-url=$(DB_URL) > src/infrastructure/src/schema.rs

cargo_fmt:
	cargo format

cargo_fmt_check:
	cargo format_check

cargo_fix:
	cargo fix_lint_warnings

cargo_lint:
	cargo lint

cargo_test:
	cargo run_tests

cargo_test_unit:
	cargo unit_tests

cargo_test_integration:
	cargo integration_tests

cargo_clean:
	rm -rf ~/.cargo/registry/index/*

cargo_all_checks: cargo_fmt_check cargo_fix cargo_lint cargo_test