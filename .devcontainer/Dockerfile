# The development container for Jarrow (docs/contributing/README.md).
#
# .github/workflows/devcontainer.yml builds this file and publishes it as
# ghcr.io/moshebyte/jarrow-devcontainer:latest and :<commit sha>. devcontainer.json opens that
# published image, so a Codespace starts with every tool below already installed.
#
# The build context is the repository root. Dockerfile.dockerignore (next to this file) cuts
# it down to the two files the image reads: rust-toolchain.toml, the one Rust pin, and
# tools/requirements-dev.txt, the Python test tools with their hashes. Build it locally with:
#   docker build -f .devcontainer/Dockerfile -t jarrow-devcontainer:local .
#   bash .devcontainer/test-image.sh jarrow-devcontainer:local
#
# Every download is pinned: the base image by digest; rustup-init, Node and gh by version and
# SHA-256; cargo-fuzz by version with --locked; the Python packages by hash. The image is
# linux/amd64 only, which is what Codespaces runs.

# mcr.microsoft.com/devcontainers/base:3.0.12-noble (Ubuntu 24.04 with the "vscode" user)
FROM mcr.microsoft.com/devcontainers/base@sha256:ec556aed93f62aa6d13fe72c5db09da4e29a7c9b87c568dbca0ee5717466926a

LABEL org.opencontainers.image.source="https://github.com/MosheByte/jarrow" \
      org.opencontainers.image.description="Development container for Jarrow: Rust, cargo-fuzz, Python 3.12, Node 24, JDK 21 and gh" \
      org.opencontainers.image.licenses="MIT"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
USER root

# Node: https://nodejs.org/dist/v24.22.2/SHASUMS256.txt
ARG NODE_VERSION=24.22.2
ARG NODE_SHA256=48cc0e21e048790e54c0cf5749c2725005313d652651c40b7a21feb9697aaf75
# gh: https://github.com/cli/cli/releases/download/v2.103.3/gh_2.103.3_checksums.txt
ARG GH_VERSION=2.103.3
ARG GH_SHA256=4c806978ffce1cec70853302a56d70ac6da4f19bae7adca4e1a8ab00633dd91d
# rustup: https://static.rust-lang.org/rustup/archive/1.29.3/x86_64-unknown-linux-gnu/rustup-init.sha256
ARG RUSTUP_VERSION=1.29.3
ARG RUSTUP_INIT_SHA256=b1c8cb14e30889e7e2999e6b69e62d9e9b9ffc0f4df8180564d306d9680e6c55
ARG CARGO_FUZZ_VERSION=0.13.2

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    VIRTUAL_ENV=/opt/jarrow-venv \
    JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 \
    PIP_DISABLE_PIP_VERSION_CHECK=1 \
    PATH=/opt/jarrow-venv/bin:/usr/local/cargo/bin:/opt/node/bin:${PATH}

# Ubuntu packages: a C and C++ toolchain (the linker Rust needs, and the compiler libFuzzer
# needs), Python 3.12 with venv, JDK 21 for the harness, xz for the Node archive. The upgrade
# picks up Ubuntu security fixes on the weekly rebuild.
RUN if [ "$(dpkg --print-architecture)" != amd64 ]; then echo "This image is linux/amd64 only" >&2; exit 1; fi \
 && apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get upgrade -y --no-install-recommends \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      build-essential \
      jq \
      openjdk-21-jdk-headless \
      pkg-config \
      python3.12 \
      python3.12-venv \
      xz-utils \
 && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL -o /tmp/node.tar.xz "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
 && echo "${NODE_SHA256}  /tmp/node.tar.xz" | sha256sum -c - \
 && mkdir -p /opt/node \
 && tar -xJf /tmp/node.tar.xz -C /opt/node --strip-components=1 --no-same-owner \
 && rm /tmp/node.tar.xz

RUN curl -fsSL -o /tmp/gh.tar.gz "https://github.com/cli/cli/releases/download/v${GH_VERSION}/gh_${GH_VERSION}_linux_amd64.tar.gz" \
 && echo "${GH_SHA256}  /tmp/gh.tar.gz" | sha256sum -c - \
 && tar -xzf /tmp/gh.tar.gz -C /tmp \
 && install -m 0755 "/tmp/gh_${GH_VERSION}_linux_amd64/bin/gh" /usr/local/bin/gh \
 && rm -rf /tmp/gh.tar.gz "/tmp/gh_${GH_VERSION}_linux_amd64"

# Rust, cargo and the Python environment belong to the "vscode" user, so the postCreateCommand
# (cargo fetch, pip install) can write to them without sudo.
RUN install -d -o vscode -g vscode "${RUSTUP_HOME}" "${CARGO_HOME}" "${VIRTUAL_ENV}"

USER vscode

RUN curl -fsSL -o /tmp/rustup-init "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
 && echo "${RUSTUP_INIT_SHA256}  /tmp/rustup-init" | sha256sum -c - \
 && chmod 0755 /tmp/rustup-init \
 && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none \
 && rm /tmp/rustup-init

# The toolchain comes from rust-toolchain.toml itself (channel, rustfmt, clippy, wasm32), the
# same way CI installs it, so the image follows a pin bump without an edit here. rust-src is
# added for rust-analyzer.
COPY --chown=vscode:vscode rust-toolchain.toml /tmp/jarrow-toolchain/rust-toolchain.toml
RUN cd /tmp/jarrow-toolchain \
 && channel="$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)" \
 && if [ -z "${channel}" ]; then echo "No channel in rust-toolchain.toml" >&2; exit 1; fi \
 && rustup toolchain install \
 && rustup default "${channel}" \
 && rustup component add --toolchain "${channel}" rust-src \
 && cd / \
 && rm -rf /tmp/jarrow-toolchain

RUN cargo install --locked cargo-fuzz --version "${CARGO_FUZZ_VERSION}" \
 && rm -rf "${CARGO_HOME}/registry" "${CARGO_HOME}/git"

COPY --chown=vscode:vscode tools/requirements-dev.txt /tmp/jarrow-requirements-dev.txt
RUN python3.12 -m venv "${VIRTUAL_ENV}" \
 && python -m pip install --no-cache-dir --require-hashes -r /tmp/jarrow-requirements-dev.txt \
 && rm /tmp/jarrow-requirements-dev.txt

# Fail the build when a tool is missing or on the wrong major version.
RUN set -eux; \
    rustc --version; \
    cargo --version; \
    cargo fmt --version; \
    cargo clippy --version; \
    rustup target list --installed | grep -qx wasm32-unknown-unknown; \
    cargo fuzz --version; \
    python --version | grep -q '^Python 3\.12\.'; \
    python -m pytest --version; \
    node --version | grep -q '^v24\.'; \
    npm --version; \
    java -version 2>&1 | grep -q 'version "21\.'; \
    gh --version

# Like the base image, end as root; devcontainer.json sets remoteUser to vscode.
USER root
