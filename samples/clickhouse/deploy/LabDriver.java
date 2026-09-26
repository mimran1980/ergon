import io.aeron.archive.ArchivingMediaDriver;
import org.agrona.CloseHelper;

/// The Aeron media driver and archive in one JVM, like `ArchivingMediaDriver`,
/// but exits when the archive's Aeron client closes. A long enough pause (the
/// host sleeping, say) makes that client time out: the archive stops for good
/// while the driver runs on, and nothing could record. Exiting lets Kubernetes
/// restart the pod, and every client reconnects to the new driver.
public final class LabDriver {
    public static void main(String[] args) throws InterruptedException {
        var driver = ArchivingMediaDriver.launch();
        // Close cleanly on SIGTERM and on the exit below. Otherwise the next
        // archive refuses to start until the old mark file goes stale.
        Runtime.getRuntime().addShutdownHook(new Thread(() -> CloseHelper.quietClose(driver)));
        var aeron = driver.archive().context().aeron();
        while (!aeron.isClosed()) {
            Thread.sleep(1000);
        }
        System.err.println("the archive's Aeron client closed; exiting so the pod restarts");
        System.exit(1);
    }
}
