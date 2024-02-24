SHELL := /bin/bash
HIDE ?= @

.PHONY: build

clean:
	$(HIDE)cargo clean

fix:
	$(HIDE)cargo fmt

build:
	$(HIDE)cargo build

publish:
	$(HIDE)cargo publish --registry crates-io

