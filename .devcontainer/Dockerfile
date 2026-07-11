FROM mcr.microsoft.com/devcontainers/rust:1

# Install GTK4, Libadwaita, dav1d, and other tools/libraries needed for the workspace
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        libgtk-4-dev \
        libadwaita-1-dev \
        libdav1d-dev \
        pkg-config \
        make \
        pre-commit \
        shellcheck \
        shfmt \
        make \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# Switch to the vscode user to install cargo tools
USER vscode

# Install additional cargo tools that are used in bootstrap/CI
RUN cargo install cargo-audit \
    && cargo install --locked cargo-deny \
    && cargo install cargo-llvm-cov
