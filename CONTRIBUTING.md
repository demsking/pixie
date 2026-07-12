# Contributing to Pixie

Thank you for your interest in contributing to Pixie! This document guides you through our development workflow.

## Before Submitting an Issue

Check that [our issue database](https://gitlab.com/demsking/oremi-isinmi-agent/issues)
doesn't already include that problem or suggestion before submitting an issue.
If you find a match, you can use the "subscribe" button to get notified on
updates. Do _not_ leave random "+1" or "I have this too" comments, as they
only clutter the discussion, and don't help resolving it. However, if you
have ways to reproduce the issue or have additional information that may help
resolving the issue, please leave a comment.

## Writing Good Bug Reports and Feature Requests

Please file a single issue per problem and feature request. Do not file combo
issues. Please do not submit multiple comments on a single issue - write your
issue with all the environmental information and reproduction steps so that an
engineer can reproduce it.

The community wants to help you find a solution to your problem, but every
problem is unique. In order for an engineer to help resolve your issue, they
need to be able to reproduce it. We put the product through extensive manual
and automated QA for every release, and verify all of its functionality. Any
feature that was previously working in a release that is no longer working is
marked as a severity/blocker for immediate review.

This means that if you are encountering an error, it is likely due to a unique
configuration of your system. Reproducing your specific error may require
significant information about your system and environment. Help us in advance
by providing a complete assessment of your system and the steps necessary to
reproduce the issue.

Therefore:

- The details of your environment including OS version, Python version.
- Provide reproducible steps, what the result of the steps was, and what you
  would have expected.
- A detailed description of the behavior that you expect.

## Developer Setup

Devbox is a command-line tool that lets you easily create isolated shells for
development.

1. **Install Devbox**
   - Follow the instructions to [install Devbox](https://www.jetify.com/devbox/docs/installing_devbox/).

2. **Install `direnv`**
   - Install `direnv` using your OS package manager. Refer to the [installation
     guide](https://direnv.net/docs/installation.html#from-system-packages).

3. **Hook `direnv` into Your Shell**
   - Follow the steps to [hook `direnv` into your shell](https://direnv.net/docs/hook.html).

4. **Clone the Repository**:

   ```bash
   git clone git@gitlab.com:demsking/pixie.git
   cd pixie
   ```

5. **Load Environment**
   - From the the project root directory, run:
     ```sh
     direnv allow
     ```
   - The next time you launch your terminal and enter the project directory,
     `direnv` will automatically load the Devbox environment.

6. **Bootstrap the Environment**:

   ```bash
   devbox run bootstrap
   ```

   This will install all necessary development tools, verify dependencies, and register git hooks.

7. **Start Environment**
   Start the development environment with:
   ```sh
   make shell
   ```
   This will launch a preconfigured Tmux session. Refer to the
   [.tmuxinator.yml](.tmuxinator.yml) file for details.

## Development Workflow

We use GNU Make to coordinate common tasks:

- **Check Code**: `make check`
- **Run Tests**: `make test-all`
- **Lint Code**: `make lint`
- **Format Code**: `make fmt`
- **Run App**: `make run`
- **Build Release**: `make build-release`

## GNU Make Commands

We expose common tasks via the root `Makefile`:

```text
make bootstrap      - Set up local dependencies & hooks
make check          - Check compilation
make test-all       - Run unit, property, and integration tests
make run            - Run Pixie desktop app
make outdated       - Check for outdated dependencies
make update         - Update dependencies
make lint           - Run Clippy checks
make fmt            - Format codebase
make security       - Run cargo-audit and cargo-deny checks
make docs-build     - Compile mdBook and API docs
make shell          - Start tmuxinator workspace
```

## Project Architecture

```text
pixie/
├── crates/
│   ├── pixie-core/   - Headless image transformations, pipeline and history tracking
│   └── pixie-app/    - GTK4 / Libadwaita window components and background workers
```

### tmux Workspace

You can launch a pre-configured tmuxinator development workspace with:

```bash
make shell
```

## Git Conventions

- **Branch Naming**: Use feature branch names like `feature/your-feature-name` or `bugfix/issue-description`.
- **Commits**: Ensure commits are cleanly grouped, pass all pre-commit hooks, and have informative messages.
- **Formatting**: All Rust code must be formatted via `cargo fmt`.

## Security Considerations

Do not commit credentials, tokens, or personal paths in code or tests. Verify files and input directories in tests do not follow symlinks insecurely.
Report vulnerabilities privately to `demsking@gmail.com` as described in [SECURITY.md](https://gitlab.com/demsking/pixie/-/blob/main/SECURITY.md).
