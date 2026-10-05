# SPDX-FileCopyrightText: 2026 Sebastien Rousseau <sebastian.rousseau@gmail.com>
# SPDX-License-Identifier: Apache-2.0 OR MIT

CARGO ?= cargo

.PHONY: all check clippy test fmt doc docs docs-serve gate demo clean

all: fmt check clippy test

check:
	$(CARGO) check --all-targets --all-features

clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test:
	$(CARGO) test --all-features

fmt:
	$(CARGO) fmt --all -- --check

doc:
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --no-deps --all-features


demo: ## Generate terminal demo GIF using VHS
	vhs .github/demo.tape

clean:
	$(CARGO) clean
