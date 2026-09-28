# The Aeron Archive, straight from Aeron's jar: a client of the node's C
# media driver, configured by JAVA_TOOL_OPTIONS (deploy/infra/aeron.yaml).
# Its main exits on SIGTERM, closing cleanly, and on any fatal Aeron error,
# such as its client timing out after the host stalled: Kubernetes then
# restarts it, and it reconnects.
FROM eclipse-temurin:21-jre
RUN wget -q -O /aeron-all.jar \
    https://repo1.maven.org/maven2/io/aeron/aeron-all/1.52.2/aeron-all-1.52.2.jar
# First `ArchiveTool verify`: a recording whose last page was half written
# when the node went down (a VM killed under memory pressure, say) stops the
# archive from starting at all, and with it everything that records on the
# node. Verify checks each recording's last segment, so it is quick, and
# asks before it truncates a fragment it cannot verify: `yes` truncates it,
# keeping the rest of the recording. It exits non-zero when it found one:
# the archive starts either way.
ENTRYPOINT ["sh", "-c", "yes y | java --add-opens java.base/jdk.internal.misc=ALL-UNNAMED -cp /aeron-all.jar \
             io.aeron.archive.ArchiveTool /archive/recordings verify; \
             exec java --add-opens java.base/jdk.internal.misc=ALL-UNNAMED -cp /aeron-all.jar io.aeron.archive.Archive"]
