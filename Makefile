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
	rsync -rltgoDv target/doc rediger:/var/www/sites/rdb.fnordig.de/
	ssh rediger 'chmod -R o+r /var/www/sites/rdb.fnordig.de/doc && find /var/www/sites/rdb.fnordig.de/doc -type d -exec chmod o+x {} \;'

upload: www-upload doc-upload

.PHONY: www
