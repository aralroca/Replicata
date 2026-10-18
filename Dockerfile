# Coordinator image (axum + x402 + crank). Migrations are embedded in the binary.
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
# The workspace release profile (fat LTO, 1 codegen unit) targets the on-chain program; a server
# does not need it and fat LTO can exhaust the memory of small builders.
ENV CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
RUN cargo build --release -p replicata-coordinator

FROM debian:bookworm-slim
# Served at /version so deploys can verify which build is live.
ARG REPLICATA_VERSION=dev
ENV REPLICATA_VERSION=$REPLICATA_VERSION
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/replicata-coordinator /usr/local/bin/replicata-coordinator
ENV BIND=0.0.0.0:8787
EXPOSE 8787
USER nobody
CMD ["replicata-coordinator"]
