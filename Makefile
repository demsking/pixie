.DEFAULT_GOAL := help

# Colors
YELLOW := \033[33m
RESET  := \033[0m

.PHONY: shell
shell: ## Launch tmuxinator workspace
	tmuxinator start pixie

.PHONY: help
help: ## Display this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(RESET) %s\n", $$1, $$2}'

.PHONY: bootstrap
bootstrap: ## Setup local development environment and dependencies
	@bash scripts/bootstrap.sh

outdated:
	cargo install cargo-outdated

update:
	cargo update

.PHONY: install-hooks
install-hooks: ## Install pre-commit and pre-push hooks
	pre-commit install
	pre-commit install --hook-type pre-push

.PHONY: build
build: ## Build the workspace in debug mode
	cargo build --workspace

.PHONY: build-release
build-release: ## Build the workspace in release mode
	cargo build --workspace --release --locked

.PHONY: run
run: ## Run the desktop application
	cargo run --package pixie-app

.PHONY: check
check: ## Check compiling of workspace
	cargo check --workspace

.PHONY: fmt
fmt: ## Format codebase
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Check formatting of codebase
	cargo fmt --all -- --check

.PHONY: lint
lint: ## Lint codebase using clippy
	cargo clippy --workspace --all-targets --all-features -- -D warnings -A clippy::collapsible-if -A clippy::needless-borrows-for-generic-args -A deprecated

.PHONY: test
test: ## Run standard unit/integration tests
	cargo test --workspace

.PHONY: test-core
test-core: ## Run pixie-core tests
	cargo test --package pixie-core

.PHONY: test-app
test-app: ## Run pixie-app tests
	cargo test --package pixie-app

.PHONY: test-all
test-all: ## Run all tests including core and app with all features
	cargo test --workspace --all-features --locked

.PHONY: coverage
coverage: ## Run test coverage check
	cargo llvm-cov --workspace --all-features --html

.PHONY: audit
audit: ## Check dependencies for security advisories
	cargo audit

.PHONY: deny
deny: ## Check licenses and duplicate dependencies
	cargo deny check

.PHONY: security
security: audit deny ## Run security audits (audit + deny)

.PHONY: pages
pages: ## Serve documentation locally
	serve pages

.PHONY: clean
clean: ## Clean cargo target folder
	cargo clean

.PHONY: distclean
distclean: clean ## Remove all temporary, build, and flatpak folders
	rm -rf public/ docs/book/ .flatpak-builder/ build/ bin/ *.log

.PHONY: flatpak
flatpak: flatpak-build ## Alias for flatpak-build

.PHONY: flatpak-build
flatpak-build: ## Build flatpak package locally
	flatpak-builder --force-clean --user --install-deps-from=flathub --repo=repo build-dir packaging/flatpak/io.gitlab.demsking.Pixie.json

.PHONY: flatpak-run
flatpak-run: ## Run flatpak app
	flatpak run io.gitlab.demsking.Pixie

.PHONY: package
package: ## Package the application
	@bash scripts/package.sh

.PHONY: bump-beta
bump-beta: ## Bump version to next beta (x.y.z-beta.N)
	@$(MAKE) version-bump BUMP_KIND=beta

.PHONY: bump-patch
bump-patch: ## Bump patch version (x.y.z -> x.y.z+1)
	@$(MAKE) version-bump BUMP_KIND=patch

.PHONY: bump-minor
bump-minor: ## Bump minor version (x.y.z -> x.y+1.0)
	@$(MAKE) version-bump BUMP_KIND=minor

.PHONY: bump-major
bump-major: ## Bump major version (x.y.z -> x+1.0.0)
	@$(MAKE) version-bump BUMP_KIND=major

.PHONY: version-bump
version-bump:
	@set -eu; \
	current=$$(sed -n 's/^version = "\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\(-beta\.[0-9][0-9]*\)\{0,1\}\(\+[A-Za-z0-9.-][A-Za-z0-9.-]*\)\{0,1\}\)"$$/\1/p' crates/pixie-app/Cargo.toml | head -n1); \
	if [ -z "$$current" ]; then \
		echo "Could not parse current version from crates/pixie-app/Cargo.toml"; \
		exit 1; \
	fi; \
	next=$$(perl -e 'my ($$v, $$kind) = @ARGV; if ($$v !~ /^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?(?:\+[0-9A-Za-z.-]+)?$$/) { die "Unsupported version format: $$v\\n"; } my ($$maj, $$min, $$pat, $$beta) = ($$1, $$2, $$3, $$4); my $$out; if ($$kind eq "beta") { $$out = defined $$beta ? "$$maj.$$min.$$pat-beta." . ($$beta + 1) : "$$maj.$$min.$$pat-beta.1"; } elsif ($$kind eq "patch") { $$out = "$$maj.$$min." . ($$pat + 1); } elsif ($$kind eq "minor") { $$out = "$$maj." . ($$min + 1) . ".0"; } elsif ($$kind eq "major") { $$out = ($$maj + 1) . ".0.0"; } else { die "Unknown bump kind: $$kind\\n"; } print $$out;' "$$current" "$(BUMP_KIND)"); \
	echo "Bumping version: $$current -> $$next"; \
	for file in crates/pixie-app/Cargo.toml crates/pixie-core/Cargo.toml; do \
		awk -v next="$$next" 'BEGIN { in_package = 0; updated = 0 } /^\[package\]$$/ { in_package = 1; print; next } in_package && !updated && /^version = "/ { print "version = \"" next "\""; updated = 1; in_package = 0; next } { print } END { if (!updated) { exit 2 } }' "$$file" > "$$file.tmp"; \
		mv "$$file.tmp" "$$file"; \
	done

.PHONY: ci
ci: fmt-check lint test-all security docs-build build-release ## Run all CI steps locally
