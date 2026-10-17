# syntax=docker/dockerfile:1
# Local artifact builder only: no service image, push, tag or deployment.
FROM rust:1.97.0-alpine3.23@sha256:ca0daf101eef0c8cd1e49dfc137154a220efb8c458c85a3eacacc7dfd5d9e04c AS build
RUN apk add --no-cache make musl-dev file
WORKDIR /src
ENV CARGO_BUILD_JOBS=1
COPY Makefile ./
COPY rust/Cargo.toml rust/Cargo.lock rust/LICENSE ./rust/
COPY rust/src ./rust/src
COPY rust/examples ./rust/examples
COPY rust/vendor ./rust/vendor
ARG SOURCE_BASE_REVISION
RUN test -n "$SOURCE_BASE_REVISION" && mkdir /out && \
    printf '%s\n' "$SOURCE_BASE_REVISION" > /out/source-base-revision.txt && \
    (find rust/src rust/examples rust/vendor -type f; printf '%s\n' Makefile rust/Cargo.toml rust/Cargo.lock rust/LICENSE) | \
    LC_ALL=C sort | xargs sha256sum > /out/source-files.sha256 && \
    sha256sum /out/source-files.sha256 > /out/source-manifest.sha256 && \
    rustc -vV > /out/toolchain.txt && \
    make build-public-validator ARGS='--locked' && \
    cp rust/target/release/openmls_public_validator /out/ && \
    file /out/openmls_public_validator > /out/binary-format.txt && \
    cd /out && sha256sum openmls_public_validator > artifact.sha256
FROM scratch AS artifact
COPY --from=build /out/ /
