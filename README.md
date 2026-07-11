# Pixie

**Modern Image Processor**

[![pipeline status](https://gitlab.com/demsking/pixie/badges/main/pipeline.svg)](https://gitlab.com/demsking/pixie/-/commits/main)
[![documentation](https://img.shields.io/badge/docs-gitlab--pages-blue)](https://demsking.gitlab.io/pixie)
[![license](https://img.shields.io/badge/license-GPL--3.0--or--later-green)](https://gitlab.com/demsking/pixie/-/blob/main/LICENSE)

Pixie is a modern, secure, and lightweight GTK4 image processor built with Rust.
It provides focused tools for cropping, resizing, rotating, flipping, and
converting images without the complexity of a complete graphics editor.

![Screenshot Placeholder](data/io.gitlab.demsking.Pixie.png)

## Features

- **Responsive GTK4 Interface**: Built using Libadwaita conforming to GNOME Human Interface Guidelines.
- **Non-Destructive Pipeline**: Edits are stored as an operation stack; full-resolution rendering is lazy.
- **Secure File Decoding**: Pre-verifies image headers to prevent decompression bombs.
- **Interactive Cropping**: Free aspect ratio, circle crop, standard ratios (1:1, 5:4, 4:3, 3:2, 16:9), and optional rule-of-thirds grid.
- **Checked Resizing**: Prevent enlargement, aspect ratio locking, and high-quality Lanczos3 interpolation.
- **Accurate Rotations**: Lossless 90/180-degree rotations, arbitrary angle rotations with canvas expansion and custom backgrounds.
- **Metadata Protection**: stripping GPS and camera details by default to protect privacy.
- **Keyboard Shortcuts**: Common operations bound to simple shortcuts.

## Supported Formats

- PNG
- JPEG
- WebP
- BMP
- TIFF
- Static GIF
- AVIF (native support)

## Installation & Setup

### 1. Devbox Setup (Recommended)

Make sure you have [Devbox](https://www.jetify.com/docs) installed.

```bash
git clone git@gitlab.com:demsking/pixie.git
cd pixie
devbox install
devbox run bootstrap
devbox shell
make check
make test
make run
```

### 2. Manual Setup (Without Devbox)

#### Fedora Setup
```bash
sudo dnf install gtk4-devel libadwaita-devel pkgconf make rustup
rustup-init
make bootstrap
```

#### Debian or Ubuntu Setup
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev pkg-config make rustup
rustup-init
make bootstrap
```

### 3. Flatpak Setup
```bash
make flatpak-build
make flatpak-run
```

## GNU Make Commands

We expose common tasks via the root `Makefile`:

```text
make bootstrap      - Set up local dependencies & hooks
make check          - Check compilation
make test-all       - Run unit, property, and integration tests
make run            - Run Pixie desktop app
make lint           - Run Clippy checks
make fmt            - Format codebase
make security       - Run cargo-audit and cargo-deny checks
make docs-build     - Compile mdBook and API docs
make tmux           - Start tmuxinator workspace
```

## tmux and tmuxinator Workflow

Start your pre-configured editor, logs, and test shell with:
```bash
make tmux
```
It utilizes [.tmuxinator.yml](https://gitlab.com/demsking/pixie/-/blob/main/.tmuxinator.yml) to launch editor, application runners, tests, and documentation servers.

### gitmux Configuration
Displays Git status in your tmux status bar. See configuration in [.gitmux.conf](https://gitlab.com/demsking/pixie/-/blob/main/.gitmux.conf).

## Development Workflow & Code Quality

Our quality checks are enforced via:
1. **Pre-commit Hooks**: Pinned pre-commit hooks validate formatting, shebangs, YAML, TOML, and JSON files on commit.
2. **GitLab CI**: GitLab CI pipelines execute tests, security scans, format checks, build targets, and mdBook pages updates.

## Project Architecture

```text
pixie/
├── crates/
│   ├── pixie-core/   - Headless image transformations, pipeline and history tracking
│   └── pixie-app/    - GTK4 / Libadwaita window components and background workers
```

## Documentation

The complete documentation is available at:

https://demsking.gitlab.io/pixie

## Repository

git@gitlab.com:demsking/pixie.git

## Author

Pixie is developed and maintained by Sébastien Demanou.
Contact: demsking@gmail.com
