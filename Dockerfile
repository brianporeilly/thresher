# syntax=docker/dockerfile:1

# ---- build stage -----------------------------------------------------
FROM rust:1-slim AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

# Cache dependencies separately from source changes.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && rm -rf src

COPY src ./src
COPY static ./static
# Touch main.rs so cargo doesn't skip the real build due to the stub above.
RUN touch src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl

# ---- runtime stage -----------------------------------------------------
# distroless/static-debian12 includes CA certs (needed for postgres+tls)
# and nothing else — a good match for a fully static musl binary.
FROM gcr.io/distroless/static-debian12

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/sift-server /sift-server

ENV DATABASE_URL=sqlite:///data/sift.db
ENV PORT=8080

EXPOSE 8080
VOLUME ["/data"]

ENTRYPOINT ["/sift-server"]
