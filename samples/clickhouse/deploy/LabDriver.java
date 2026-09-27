import io.aeron.archive.Archive;
import org.agrona.CloseHelper;

/// The node's Aeron Archive, a client of the node's C media driver (the
/// `driver` container), configured by system properties (see lab.yaml). It
/// exits when its Aeron client closes: the driver has gone, or a long pause
/// (the host sleeping, say) timed the client out, and the archive would
/// otherwise stay stopped for good. Exiting lets Kubernetes restart it, and
/// it reconnects to the driver.
public final class LabDriver {
    public static void main(String[] args) throws InterruptedException {
        var archive = Archive.launch(new Archive.Context());
        // Close cleanly on SIGTERM and on the exit below. Otherwise the next
        // archive refuses to start until the old mark file goes stale.
        Runtime.getRuntime().addShutdownHook(new Thread(() -> CloseHelper.quietClose(archive)));
        var aeron = archive.context().aeron();
        while (!aeron.isClosed()) {
            Thread.sleep(1000);
        }
        System.err.println("the archive's Aeron client closed; exiting so the pod restarts");
        System.exit(1);
    }
}
