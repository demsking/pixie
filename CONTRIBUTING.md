# Contributing to Pixie

Thank you for your interest in contributing to Pixie! This document guides you through our development workflow.

## Developer Setup

1. **Install Devbox**: Refer to [Jetify Devbox Docs](https://www.jetify.com/docs).
2. **Clone the Repository**:
   ```bash
   git clone git@gitlab.com:demsking/pixie.git
   cd pixie
   ```
3. **Bootstrap the Environment**:
   ```bash
   devbox run bootstrap
   ```
   This will install all necessary development tools, verify dependencies, and register git hooks.

## Development Workflow

We use GNU Make to coordinate common tasks:

- **Check Code**: `make check`
- **Run Tests**: `make test-all`
- **Lint Code**: `make lint`
- **Format Code**: `make fmt`
- **Run App**: `make run`
- **Build Release**: `make build-release`

### tmux Workspace

You can launch a pre-configured tmuxinator development workspace with:
```bash
make tmux
```

## Git Conventions

- **Branch Naming**: Use feature branch names like `feature/your-feature-name` or `bugfix/issue-description`.
- **Commits**: Ensure commits are cleanly grouped, pass all pre-commit hooks, and have informative messages.
- **Formatting**: All Rust code must be formatted via `cargo fmt`.

## Security Considerations

Do not commit credentials, tokens, or personal paths in code or tests. Verify files and input directories in tests do not follow symlinks insecurely.
Report vulnerabilities privately to `demsking@gmail.com` as described in [SECURITY.md](https://gitlab.com/demsking/pixie/-/blob/main/SECURITY.md).
