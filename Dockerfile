FROM lukemathwalker/cargo-chef:latest-rust-1.65.0 as chef
WORKDIR /app

FROM chef as planner
COPY . .
# Compute a lock-file
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin backdrop

# Runtime stage
FROM debian:bullseye-slim AS runtime
WORKDIR /app

RUN apt-get update -y \
	&& apt-get install -y --no-install-recommends openssl ca-certificates \
	&& apt-get install -y ffmpeg \
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/backdrop backdrop
COPY configuration configuration
COPY templates templates

ENV APP_ENVIRONMENT production
ENTRYPOINT ["./backdrop"]