test: dump-test unit-test

dump-test:
	./tests/dump-tests.sh

unit-test:
	cargo test

www:
	make -C www

www-upload:
	make -C www upload
