# Runtime image for both processes: `just build-recorder` compiles the Linux
# binaries first. The schema ships with them, so they always agree.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libbsd0 \
    && rm -rf /var/lib/apt/lists/*
COPY recorder ingester /usr/local/bin/
COPY market.xml /lab/schema/market.xml
ENTRYPOINT ["/usr/local/bin/recorder"]
