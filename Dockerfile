# syntax=docker/dockerfile:1

# ---- build stage -----------------------------------------------------
FROM rust:1-slim AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set natively by buildx per platform (amd64/arm64) — each is built on a
# runner matching that arch, so this is always a native, non-cross build.
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-musl ;; \
      arm64) echo aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac > /rust_target.txt \
    && rustup target add "$(cat /rust_target.txt)"

WORKDIR /app

# Cache dependencies separately from source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked --target "$(cat /rust_target.txt)" \
    && rm -rf src

COPY src ./src
COPY static ./static
# Touch main.rs so cargo doesn't skip the real build due to the stub above.
RUN touch src/main.rs \
    && cargo build --release --locked --target "$(cat /rust_target.txt)" \
    && cp "target/$(cat /rust_target.txt)/release/thresher" /thresher

# ---- runtime stage -----------------------------------------------------
# distroless/static-debian12 includes CA certs (needed for postgres+tls)
# and nothing else — a good match for a fully static musl binary. The
# :nonroot variant runs as uid/gid 65532 instead of root.
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /thresher /thresher

ENV DATABASE_URL=sqlite:///data/thresher.db
ENV PORT=8080

EXPOSE 8080
VOLUME ["/data"]

ENTRYPOINT ["/thresher"]
