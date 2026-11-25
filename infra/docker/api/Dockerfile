FROM rust:1.99.1-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends make perl \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add rustfmt clippy
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache third-party dependencies across application and test changes.
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && printf '\n' > src/lib.rs \
    && cargo build --locked
COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs src/lib.rs migrations/*.sql \
    && cargo build --locked
COPY tests ./tests
EXPOSE 8000
CMD ["cargo", "run", "--locked"]
