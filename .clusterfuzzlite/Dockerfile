# ClusterFuzzLite build image for Jarrow's cargo-fuzz targets (fuzz/).
# See https://google.github.io/clusterfuzzlite/build-integration/rust-lang/
#
# The base image is pinned by digest (tag v1, resolved 2026-11-03).
FROM gcr.io/oss-fuzz-base/base-builder-rust:v1@sha256:3428d0a90b1e7efbe2ba947bd1d0c9a70594986b4b6553804547f1fb24cfc92f

# The base image's toolchain (RUSTUP_TOOLCHAIN=nightly-2025-09-05, rustc 1.91) is older than
# the workspace MSRV, 1.99 (rust-toolchain.toml). cargo fuzz needs a nightly for the
# sanitizer flags, so install a dated nightly that is newer than 1.99 and make it the
# active toolchain. RUSTUP_TOOLCHAIN also overrides rust-toolchain.toml.
RUN rustup toolchain install nightly-2026-10-15 --profile minimal --component rust-src
ENV RUSTUP_TOOLCHAIN=nightly-2026-10-15

COPY . $SRC/jarrow
WORKDIR $SRC/jarrow
COPY .clusterfuzzlite/build.sh $SRC/
