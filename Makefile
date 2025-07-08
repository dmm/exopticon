
WEB_SRC_FILES := $(shell find exopticon/web/ -path exopticon/web/src/node_modules -prune -type f -name '*.ts' -o -name '*.css' -o -name '*.json')

WEB_OBJ_FILES := exopticon/web/dist/browser/main-*.js

.PHONY: all
all: development

.PHONY: development
development: target/debug/exopticon

.PHONY: release
release: target/release/exopticon

.PHONY: ci-flow
ci-flow:
	$(MAKE) check-format
	$(MAKE) build-web
	$(MAKE) clippy
	$(MAKE) target/release/exopticon

.PHONY: run
run:
	cargo build
	cargo run

.PHONY: watch-web
watch-web:
	cd exopticon/web; npm install
	cd exopticon/web;	npm run build -- --deploy-url assets/ --watch

.PHONY: build-web
build-web:
	$(MAKE) $(WEB_OBJ_FILES)

$(WEB_OBJ_FILES): $(WEB_SRC_FILES)
	cd exopticon/web; npm install
	cd exopticon/web; npm run check-format
	cd exopticon/web; npm run ng build -- --configuration=production --deploy-url assets/

.PHONY: check-web
check-web:
	cd exopticon/web; npm install
	cd exopticon/web; npm run check-format

.PHONY: clippy
clippy:
	cargo clippy

.PHONY: check-format
check-format:
	cargo fmt --check
	cd exopticon/web; npm run check-format

.PHONY: target/debug/exopticon
target/debug/exopticon: build-web
	cargo build

.PHONY: target/release/exopticon
target/release/exopticon: build-web
	cargo build --release

.PHONY: clean
clean:
	rm -rf exopticon/web/dist
	cargo clean
