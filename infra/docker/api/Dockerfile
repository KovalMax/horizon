FROM rust:1.99.1-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends make perl \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY migrations ./migrations
RUN cargo build --locked
EXPOSE 8000
CMD ["cargo", "run", "--locked"]
