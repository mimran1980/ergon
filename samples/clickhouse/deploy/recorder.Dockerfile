# Runtime image only: `just build-recorder` compiles the Linux binary first.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY recorder /usr/local/bin/recorder
ENTRYPOINT ["/usr/local/bin/recorder"]
