# Coordinator image (axum + x402 + crank). Migrations are embedded in the binary.
FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p replicata-coordinator

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/replicata-coordinator /usr/local/bin/replicata-coordinator
ENV BIND=0.0.0.0:8787
EXPOSE 8787
USER nobody
CMD ["replicata-coordinator"]
