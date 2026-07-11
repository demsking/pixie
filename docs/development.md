# Development Guide

This guide describes how to compile, test, and contribute to Pixie.

## Reproducible Environment (Devbox)

We use **Devbox** to declare project dependencies:
- Install Devbox: [docs.jetify.com](https://www.jetify.com/docs)
- Start shell:
  ```bash
  devbox shell
  ```
  Devbox automatically installs Rust, cargo-audit, cargo-deny, pkg-config, GTK4, and Libadwaita development libraries inside the shell container.

## GNU Make Task List

Common developer tasks are mapped to `Makefile` targets:
- `make bootstrap`: Prepares the development environment.
- `make check`: Verifies compilation.
- `make lint`: Runs Clippy checking with strict warnings.
- `make test-all`: Runs all unit and integration tests.
- `make run`: Compiles and runs the GTK application.
- `make docs-build`: Compiles the mdBook and API docs into `public/`.
- `make flatpak-build`: Compiles and builds the Flatpak package locally.
- `make ci`: Runs all CI steps locally.

## gitmux Configuration

To integrate Pixie git status into your tmux status bar, you can add this snippet to your `~/.tmux.conf`:

```tmux
set -g status-right '#(gitmux "#{pane_current_path}")'
```

It uses our local [.gitmux.conf](https://gitlab.com/demsking/pixie/-/blob/main/.gitmux.conf) to display staged/unstaged changes, current branch, and stash count.

## pre-commit Hooks

Pre-commit hooks are configured in [.pre-commit-config.yaml](https://gitlab.com/demsking/pixie/-/blob/main/.pre-commit-config.yaml). Install them with:
```bash
make install-hooks
```
They will format all Rust code and check syntax on each local commit.
