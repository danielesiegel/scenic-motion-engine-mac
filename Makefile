# scenic-motion-engine Makefile
# Common dev + release workflows. Run `make help` for the target list.

SHELL := /bin/bash
.DEFAULT_GOAL := help

# User passes extra args as `make <target> ARGS="..."`
ARGS ?=

.PHONY: help run render build test check fmt bench clean universal bundle analyze validate-analysis install-hooks

help: ## Print this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

run: ## Launch the GUI app (cargo run --release)
	cargo run --release --bin scenic-motion-engine

render: ## Headless MP4 export: ARGS="--audio ... --out ..."
	cargo run --release --bin render -- $(ARGS)

build: ## Release build
	cargo build --release

test: ## Run cargo test
	cargo test

check: ## clippy -D warnings + fmt --check
	cargo clippy --all-targets -- -D warnings && cargo fmt -- --check

fmt: ## Apply cargo fmt to the workspace
	cargo fmt --all

bench: ## Run criterion benches
	cargo bench

clean: ## cargo clean
	cargo clean

universal: ## Build macOS universal (aarch64 + x86_64) binary
	./scripts/build_universal.sh

bundle: universal ## Build universal binary, .app bundle, and .dmg
	./scripts/bundle_app.sh

analyze: ## Analyze audio: ARGS="--input assets/audio/foo.wav"
	python3 scripts/analyze.py $(ARGS)

validate-analysis: ## Validate an analysis JSON: ARGS="path/to/foo.json"
	python3 scripts/validate_analysis.py $(ARGS)

install-hooks: ## Install the pre-commit hook into .git/hooks/
	@if [ ! -d .git ]; then echo "error: not a git repo"; exit 1; fi
	@if [ ! -f .githooks/pre-commit ]; then echo "error: .githooks/pre-commit missing"; exit 1; fi
	cp .githooks/pre-commit .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit
	@echo "installed .git/hooks/pre-commit"
