# The Aeron Archive (LabDriver.java), a client of the node's C media driver. Settings come
# from JAVA_TOOL_OPTIONS (see lab.yaml).
FROM eclipse-temurin:21-jdk AS build
RUN wget -q -O /aeron-all.jar \
    https://repo1.maven.org/maven2/io/aeron/aeron-all/1.52.2/aeron-all-1.52.2.jar
COPY LabDriver.java /src/
RUN javac -cp /aeron-all.jar -d /classes /src/LabDriver.java

FROM eclipse-temurin:21-jre
COPY --from=build /aeron-all.jar /aeron-all.jar
COPY --from=build /classes /classes
ENTRYPOINT ["java", "--add-opens", "java.base/jdk.internal.misc=ALL-UNNAMED", \
            "-cp", "/aeron-all.jar:/classes", "LabDriver"]
