.DEFAULT_GOAL := help
APP_MANIFEST := crates/pixie-app/Cargo.toml
CORE_MANIFEST := crates/pixie-core/Cargo.toml

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
install-hooks: ## Install pre-commit
	pre-commit install

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

flatpak-install: ## Install flatpak package locally
	flatpak-builder --force-clean --user --install --install-deps-from=flathub --repo=repo build-dir packaging/flatpak/io.gitlab.demsking.Pixie.json

.PHONY: flatpak-run
flatpak-run: ## Run flatpak app
	flatpak run io.gitlab.demsking.Pixie

.PHONY: package
package: ## Package the application
	@bash scripts/package.sh

.PHONY: check-clean
check-clean:
	@test -z "$$(git status --porcelain)" || (echo "Working tree is not clean" && exit 1)

.PHONY: bump
bump:
	@set -eu; \
	current="$$(sed -nE 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$(APP_MANIFEST)" | head -n 1)"; \
	if [ -z "$$current" ]; then \
		echo "Could not parse current version from $(APP_MANIFEST)" >&2; \
		exit 1; \
	fi; \
	new_version="$$(perl -e '\
		my ($$version, $$kind) = @ARGV; \
		$$version =~ /^(\d+)\.(\d+)\.(\d+)(?:-beta(?:\.(\d+))?)?(?:\+[0-9A-Za-z.-]+)?$$/ \
			or die "Unsupported version format: $$version\n"; \
		my ($$major, $$minor, $$patch, $$beta) = ($$1, $$2, $$3, $$4); \
		if ($$kind eq "beta") { \
			print defined $$beta \
				? "$$major.$$minor.$$patch-beta." . ($$beta + 1) \
				: "$$major.$$minor.$$patch-beta.1"; \
		} elsif ($$kind eq "stable") { \
      die "Version is already stable: $$version\n" \
        unless $$version =~ /-beta(?:\.\d+)?(?:\+[0-9A-Za-z.-]+)?$$/; \
      print "$$major.$$minor.$$patch"; \
    } elsif ($$kind eq "patch") { \
			print "$$major.$$minor." . ($$patch + 1); \
		} elsif ($$kind eq "minor") { \
			print "$$major." . ($$minor + 1) . ".0"; \
		} elsif ($$kind eq "major") { \
			print ($$major + 1) . ".0.0"; \
		} elsif ($$kind eq "major") { \
      print ($$major + 1) . ".0.0"; \
    } else { \
      die "Unknown bump kind: $$kind\n"; \
    } \
	' "$$current" "$(BUMP_KIND)")"; \
	echo "Bumping version: $$current -> $$new_version"; \
	for file in "$(APP_MANIFEST)" "$(CORE_MANIFEST)"; do \
		tmp="$$(mktemp "$${file}.XXXXXX")"; \
		trap 'rm -f "$$tmp"' EXIT INT TERM; \
		awk -v new_version="$$new_version" '\
			BEGIN { in_package = 0; updated = 0 } \
			/^\[package\][[:space:]]*$$/ { \
				in_package = 1; \
				print; \
				next; \
			} \
			in_package && !updated && /^[[:space:]]*version[[:space:]]*=/ { \
				print "version = \"" new_version "\""; \
				updated = 1; \
				in_package = 0; \
				next; \
			} \
			{ print } \
			END { \
				if (!updated) \
					exit 2; \
			} \
		' "$$file" > "$$tmp" || { \
			echo "Could not update version in $$file" >&2; \
			exit 1; \
		}; \
		mv "$$tmp" "$$file"; \
		trap - EXIT INT TERM; \
	done

.PHONY: beta
beta: check-clean ## Bump version to next beta (x.y.z-beta.N)
	@$(MAKE) bump BUMP_KIND=beta

.PHONY: stable
stable: check-clean ## Promote beta version to stable (x.y.z-beta.N -> x.y.z)
	@$(MAKE) bump BUMP_KIND=stable

.PHONY: patch
patch: check-clean ## Bump patch version (x.y.z -> x.y.z+1)
	@$(MAKE) bump BUMP_KIND=patch

.PHONY: minor
minor: check-clean ## Bump minor version (x.y.z -> x.y+1.0)
	@$(MAKE) bump BUMP_KIND=minor

.PHONY: major
major: check-clean ## Bump major version (x.y.z -> x+1.0.0)
	@$(MAKE) bump BUMP_KIND=major

.PHONY: release
release: check-clean
	@set -eu; \
	version="$$(sed -nE 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$(APP_MANIFEST)" | head -n 1)"; \
	if [ -z "$$version" ]; then \
		echo "Could not parse version from $(APP_MANIFEST)" >&2; \
		exit 1; \
	fi; \
  cargo build; \
	git add "$(APP_MANIFEST)" "$(CORE_MANIFEST)" Cargo.lock; \
	git commit -m "chore(release): bump version to $$version"; \
	git tag --annotate "v$$version" --message "Release v$$version"

publish: release ## Publish the package to crates.io
	git push origin main
	git push origin "v$(shell sed -n 's/^version = "\(.*\)"/\1/p' $(APP_MANIFEST))"

.PHONY: ci
ci: fmt-check lint test-all security build-release ## Run all CI steps locally
