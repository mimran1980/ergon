import io.aeron.cluster.ClusteredMediaDriver;
import io.aeron.cluster.service.*;
import io.aeron.samples.cluster.ClusterConfig;
import io.aeron.samples.cluster.tutorial.BasicAuctionClusteredService;
import io.aeron.logbuffer.Header;
import io.aeron.ExclusivePublication;
import io.aeron.Image;
import io.aeron.cluster.codecs.CloseReason;
import org.agrona.DirectBuffer;
import java.util.*;
import java.util.concurrent.TimeUnit;

public class ClusterLauncher {
    public static class Echo implements ClusteredService {
        private Cluster cluster;
        public void onStart(Cluster c, Image s) { this.cluster = c; }
        public void onSessionOpen(ClientSession s, long ts) {}
        public void onSessionClose(ClientSession s, long ts, CloseReason r) {}
        public void onSessionMessage(ClientSession s, long ts, DirectBuffer b, int off, int len, Header h) {
            while (s.offer(b, off, len) < 0) { cluster.idleStrategy().idle(); }
        }
        public void onTimerEvent(long cid, long ts) {}
        public void onTakeSnapshot(ExclusivePublication p) {}
        public void onRoleChange(Cluster.Role r) {}
        public void onTerminate(Cluster c) {}
        public void onNewLeadershipTermEvent(long t, long lp, long ts, long tb, int lm, int ls, TimeUnit tu, int av) {}
    }

    public static void main(String[] args) throws Exception {
        int basePort = Integer.parseInt(args[0]);
        int memberId = Integer.parseInt(args[1]);
        int nodeCount = Integer.parseInt(args[2]);
        List<String> hosts = new ArrayList<>();
        for (int i = 0; i < nodeCount; i++) hosts.add("localhost");

        // Optional args after the node triple: "keep" preserves archive and
        // consensus dirs (log-recovery restart). "auction" runs the official
        // sample service; anything else, including omission, is the echo service.
        // The media driver directory is always recreated: a leftover cnc.dat
        // from a killed process triggers ActiveDriverException.
        boolean keep = false;
        ClusteredService service = new Echo();
        for (int i = 3; i < args.length; i++) {
            if (args[i].equals("keep")) {
                keep = true;
            } else if (args[i].equals("auction")) {
                service = new BasicAuctionClusteredService();
            } else if (!args[i].equals("echo")) {
                throw new IllegalArgumentException("unknown ClusterLauncher arg: " + args[i]);
            }
        }
        ClusterConfig config = ClusterConfig.create(memberId, hosts, basePort, service);
        config.mediaDriverContext().dirDeleteOnStart(true);
        config.archiveContext().deleteArchiveOnStart(!keep);
        // Per-node ingress port: portBase + memberId*100 + 2 (matches
        // ClusterConfig.calculatePort with PORTS_PER_NODE=100).
        int ingressPort = basePort + memberId * 100 + 2;
        config.consensusModuleContext()
            .deleteDirOnStart(!keep)
            .ingressChannel("aeron:udp?endpoint=localhost:" + ingressPort);

        ClusteredMediaDriver.launch(config.mediaDriverContext(), config.archiveContext(), config.consensusModuleContext());
        ClusteredServiceContainer.launch(config.clusteredServiceContext());

        System.out.println("CLUSTER_READY memberId=" + memberId);
        System.out.println("INGRESS=aeron:udp?endpoint=localhost:" + ingressPort);
        System.out.println("EGRESS=aeron:udp?endpoint=localhost:" + ingressPort);
        System.out.println("AERON_DIR=" + config.mediaDriverContext().aeronDirectoryName());
        System.out.flush();
        Thread.sleep(Long.MAX_VALUE);
    }
}
