# Wraps a release binary; the image runs the same binary as the release assets.
# 24.04: the release builds on 24.04 and links its glibc.
FROM ubuntu:24.04

# The API verifies TLS certificates with the system store.
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

ARG BINARY
ARG VERSION
COPY --chmod=755 ${BINARY} /usr/local/bin/unipept-api

LABEL org.opencontainers.image.source=https://github.com/unipept/unipept-api
LABEL org.opencontainers.image.description="Unipept API"
LABEL org.opencontainers.image.licenses=MIT
LABEL org.opencontainers.image.version=${VERSION}

ENTRYPOINT ["/usr/local/bin/unipept-api"]
