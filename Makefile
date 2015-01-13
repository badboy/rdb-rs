test: dump-test unit-test

dump-test:
	./tests/dump-tests.sh

unit-test:
	cargo test

www:
	$(MAKE) -C www

www-upload:
	$(MAKE) -C www upload

doc:
	cargo doc

doc-upload: doc
	rsync -av target/doc rediger:/var/www/sites/rdb.fnordig.de/

.PHONY: www
