# Aeron media driver + archive in one JVM. Settings come from
# JAVA_TOOL_OPTIONS (see lab.yaml).
FROM eclipse-temurin:21-jre
RUN wget -q -O /aeron-all.jar \
    https://repo1.maven.org/maven2/io/aeron/aeron-all/1.52.2/aeron-all-1.52.2.jar
ENTRYPOINT ["java", "--add-opens", "java.base/jdk.internal.misc=ALL-UNNAMED", \
            "-cp", "/aeron-all.jar", "io.aeron.archive.ArchivingMediaDriver"]
