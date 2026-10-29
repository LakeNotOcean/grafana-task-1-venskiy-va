FROM rust:1.99-alpine AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin items-service

FROM alpine:3.22
RUN adduser -D -u 10001 app
COPY --from=build /app/target/release/items-service /usr/local/bin/items-service
USER app
EXPOSE 8080
HEALTHCHECK --interval=5s --timeout=2s --start-period=10s --retries=12 \
    CMD wget -qO- http://127.0.0.1:8080/health || exit 1
CMD ["items-service"]
