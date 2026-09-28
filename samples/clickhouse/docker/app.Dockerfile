# Runtime image for the lab's own binaries: `just build` compiles them for
# Linux first. Every schema in schema/ ships with them, so they always agree.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libbsd0 \
    && rm -rf /var/lib/apt/lists/*
COPY md ingester aeron-driver engine exch-sim /usr/local/bin/
COPY schema/ /lab/schema/
