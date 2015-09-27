PREFIX?=/usr
INSTALL_BIN=$(PREFIX)/bin
INSTALL=install
RDB_INSTALL=$(QUIET_INSTALL)$(INSTALL)

BINARY_PATH=./target/release/rdb

ifndef V
	QUIET_INSTALL = @printf '    %b %b\n' $(LINKCOLOR)INSTALL$(ENDCOLOR) $(BINCOLOR)$@$(ENDCOLOR) 1>&2;
endif

LINKCOLOR="\033[34;1m"
SRCCOLOR="\033[33m"
BINCOLOR="\033[37;1m"
MAKECOLOR="\033[32;1m"
ENDCOLOR="\033[0m"

build:
	cargo build

build-release:
	cargo build --release

test: dump-test eq-test unit-test

dump-test: build
	./tests/dump-tests.sh

eq-test: build
	./tests/eq-tests.sh

unit-test: build
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

install: build-release
	@mkdir -p $(INSTALL_BIN)
	$(RDB_INSTALL) $(BINARY_PATH) $(INSTALL_BIN)

tidy:
	rustfmt src/lib.rs

clean:
	find . -name "*.bk" -delete

dist-clean: clean
	rm -r target

.PHONY: www clean dist-clean tidy install upload

