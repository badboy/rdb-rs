test: dump-test unit-test

dump-test:
	./tests/dump-tests.sh

unit-test:
	cargo test
