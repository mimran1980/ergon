#!/usr/bin/env bash
# End-to-end checks against the running lab (`just up` first).
#
#  1. ClickHouse answers, and its /play UI is served.
#  2. Every deployed exchange's feed handler (md) is writing trades and quotes.
#     Every region's engine publishes EMAs and its aggregated book, its orders
#     are filled, and a kept tick-to-trade trace joins its exchange's trace.
#  3. Every Grafana panel's query runs through Grafana without error.
#  4. Metrics, histograms, traces and Aeron's counters arrive, with host and pod.
#  5. The verification notebook runs clean inside JupyterLab's pod.
#  6. Toggling a dynamic table in config/tables.yaml, published as the
#     cluster's ConfigMap (`just config`), starts and stops it live.
#  7. A feed handler moved to another node (cordon, delete): it publishes from
#     there, its engine resyncs it, its region's other feed never stops, and
#     no row is recorded twice.
#
# Leaves config/tables.yaml, and every node's schedulability, as it found them.
set -euo pipefail
cd "$(dirname "$0")/.."

CH=http://localhost:8123
GRAFANA=http://localhost:3000
KUBE=(kubectl --context kind-clickhouse-lab -n lab)
fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "ok:   $*"; }
sql() { curl -sf -u lab:lab "$CH/" --data-binary "$1"; }

# 1. ClickHouse + UI
[[ $(sql "SELECT 1") == 1 ]] || fail "ClickHouse not answering on $CH"
play=$(curl -sf "$CH/play") && [[ $play == *"<title>ClickHouse Query</title>"* ]] || fail "$CH/play did not serve the query UI"
ok "ClickHouse answers; query UI at $CH/play"

# 1b. The cluster runs this checkout's config/: `just config` publishes it.
for f in tables.yaml streams.yaml; do
    live=$("${KUBE[@]}" get configmap lab-config -o jsonpath="{.data.${f/./\\.}}")
    [[ $live == "$(cat config/$f)" ]] || fail "the cluster's $f differs from config/$f: run \`just config\`"
done
ok "the cluster's ConfigMap lab-config is config/"

# 2. Live data from every deployed exchange
expected=$("${KUBE[@]}" get deploy -l app=md -o jsonpath='{range .items[*]}{.metadata.labels.exchange}{"\n"}{end}' | tr a-z A-Z | sort | paste -sd, -)
[[ -n $expected ]] || fail "no feed handler is deployed (just deploy)"
venues=$(sql "SELECT arrayStringConcat(arraySort(groupUniqArray(venue)), ',') FROM market.trade WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL 1 MINUTE")
[[ $venues == "$expected" ]] || fail "trades in the last minute came from '$venues', expected $expected"
quotes=$(sql "SELECT count() FROM market.quote WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL 1 MINUTE")
(( quotes > 0 )) || fail "no quotes in the last minute"
ok "trades from $venues and $quotes quotes in the last minute"

# 2a. Every publisher's name resolves to its node's media driver: that is
# where its feeds' control sockets are, not wherever its pod's own network
# would put it.
drivers=$("${KUBE[@]}" get pod -l app=aeron -o jsonpath='{range .items[*]}{.spec.nodeName} {.status.podIP}{"\n"}{end}')
driver_on() { awk -v n="$1" '$1 == n { print $2 }' <<<"$drivers"; }
asker=$("${KUBE[@]}" get pod -l app=engine -o jsonpath='{.items[0].metadata.name}')
checked_names=0
while read -r service node; do
    resolved=$("${KUBE[@]}" exec "$asker" -c engine -- getent ahostsv4 "$service.lab.svc.cluster.local" | awk 'NR == 1 { print $1 }')
    driver_ip=$(driver_on "$node")
    [[ -n $driver_ip ]] || fail "no Aeron driver on $node, where $service runs"
    [[ $resolved == "$driver_ip" ]] || fail "$service resolves to '$resolved', not $node's media driver at $driver_ip"
    checked_names=$((checked_names + 1))
done < <("${KUBE[@]}" get pod -l 'app in (md,engine,exch-sim)' --field-selector=status.phase=Running \
    -o jsonpath='{range .items[*]}{.metadata.labels.app}-{.metadata.labels.exchange}{.metadata.labels.region} {.spec.nodeName}{"\n"}{end}')
ok "$checked_names publisher names resolve to the media driver of the node each runs on"

# 2b. Engines and their dummy exchanges
engines=$("${KUBE[@]}" get deploy -l app=engine -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}' | sort)
[[ -n $engines ]] || fail "no engine is deployed (just deploy)"
for engine in $engines; do
    region=${engine#engine-}
    has_md=$(sed -n "s/^  md-\([a-z]*\): .*region: $region[,} ].*/\1/p" config/streams.yaml \
        | while read -r x; do "${KUBE[@]}" get deploy "md-$x" -o name 2>/dev/null; done)
    [[ -n $has_md ]] || continue # no feed deployed in its region
    n=$(sql "SELECT uniqExact(asset) FROM market.ema WHERE app = '$engine' AND ts > now() - INTERVAL 1 MINUTE")
    (( n >= 2 )) || fail "$engine: EMAs for $n assets in the last minute, expected BTC and ETH"
    n=$(sql "SELECT count() FROM market.agg_book WHERE app = '$engine' AND ts > now() - INTERVAL 1 MINUTE AND length(bids.price) > 0 AND length(asks.price) > 0")
    (( n > 0 )) || fail "$engine: no aggregated book in the last minute"
done
ok "engines publishing EMAs and aggregated books: $(paste -sd' ' - <<<"$engines")"
orders=$(sql "SELECT count() FROM market.new_order WHERE ts > now() - INTERVAL 1 HOUR AND ts < now() - INTERVAL 30 SECOND")
(( orders > 0 )) || fail "no orders in the last hour (the strategy trades a 5m EMA cross at most every 30 s)"
# An order sent while its exchange was down is lost (plain subscriptions,
# no catch-up): judge those sent since both were last started.
unfilled=0
for engine in $engines; do
    since=$("${KUBE[@]}" get pod -l "app in (engine,exch-sim),region=${engine#engine-}" -o jsonpath='{range .items[*]}{.status.containerStatuses[0].state.running.startedAt}{"\n"}{end}' | sort | tail -1)
    [[ -n $since ]] || continue
    n=$(sql "SELECT count() FROM market.new_order WHERE app = '$engine' AND ts > parseDateTime64BestEffort('$since') AND ts < now() - INTERVAL 30 SECOND AND order_id NOT IN (SELECT order_id FROM market.execution_report WHERE status = 'Filled')")
    unfilled=$((unfilled + n))
done
[[ $unfilled == 0 ]] || fail "$unfilled orders sent while their engine and exchange were both up have no fill"
joined=$(sql "SELECT count() FROM (SELECT TraceId FROM market.otel_traces WHERE ParentSpanId = '' AND SpanName IN ('tick_to_trade', 'order_ack') AND SpanAttributes['why'] = 'kept' AND Timestamp > now() - INTERVAL 1 HOUR GROUP BY TraceId HAVING uniqExact(SpanName) = 2)")
(( joined > 0 )) || fail "no order's tick_to_trade trace shares its id with the exchange's order_ack"
ok "$orders orders in the last hour, every one sent to a running exchange filled; $joined traced from tick to exchange ack under one id"

# 3. Grafana panels
uids=$(curl -sf "$GRAFANA/api/search?type=dash-db" | jq -r '.[].uid')
[[ -n $uids ]] || fail "Grafana has no dashboards"
checked=0
tables=$(sql "SELECT name FROM system.tables WHERE database = 'market' ORDER BY name")
disabled() { grep -Eq "^  $1: .*enabled: false" config/tables.yaml; }
for uid in $uids; do
    dash=$(curl -sf "$GRAFANA/api/dashboards/uid/$uid" | jq '.dashboard')
    adhoc=$(echo "$dash" | jq -r '.templating.list[]? | select(.name=="sql") | .query // empty')
    # Query variables ($app, $trace, $trace_id) take their first value, or,
    # as `${x:singlequote}`, all of them.
    # ($table is checked for every table, below.)
    vars=$(echo "$dash" | jq -c '.templating.list[]? | select(.type=="query" and .name!="table") | {name, query}')
    while IFS= read -r target; do
        title=$(echo "$target" | jq -r '.title')
        raw=$(echo "$target" | jq -r '.rawSql')
        raw=${raw//'${sql:raw}'/$adhoc}
        while IFS= read -r v; do
            [[ -n $v ]] || continue
            name=$(echo "$v" | jq -r '.name')
            values=$(sql "$(echo "$v" | jq -r '.query') FORMAT TSV")
            raw=${raw//"\${$name:singlequote}"/$(sed "s/.*/'&'/" <<<"$values" | paste -sd, -)}
            raw=${raw//"\${$name}"/$(head -1 <<<"$values")}
        done <<<"$vars"
        title=${title//'${trace_id}'/one trace}
        # Panels driven by the $table picker are checked for every table.
        for table in $([[ $raw == *'${table'* ]] && echo "$tables" || echo "-"); do
            query=${raw//'${table:singlequote}'/"'$table'"}
            query=${query//'${table}'/$table}
            body=$(jq -n --arg sql "$query" --argjson fmt "$(echo "$target" | jq '.format')" \
                '{queries: [{refId: "A", datasource: {type: "grafana-clickhouse-datasource", uid: "clickhouse"},
                  editorType: "sql", rawSql: $sql, format: $fmt}], from: "now-30m", to: "now"}')
            result=$(curl -s -H 'Content-Type: application/json' "$GRAFANA/api/ds/query" -d "$body")
            err=$(echo "$result" | jq -r '.results.A.error // empty')
            [[ -z $err ]] || fail "Grafana panel '$title' [$table] ($uid): $err"
            rows=$(echo "$result" | jq '[.results.A.frames[]?.data.values[0]? // [] | length] | add // 0')
            # A disabled table may legitimately have nothing recent, a
            # healthy driver no errors or losses, and a lab whose tables all
            # exist no schema change within the query log's day.
            [[ $rows -gt 0 ]] || disabled "$table" || { [[ $title == *book_snapshot* ]] && disabled book_snapshot; } \
                || [[ $title == "Distinct errors" || $title == "Data loss" ]] \
                || [[ $title == "Schema changes (CREATE / ALTER)" ]] \
                || [[ $table == aeron_errors || $table == aeron_loss ]] \
                || fail "Grafana panel '$title' [$table] ($uid) returned no rows"
            checked=$((checked + 1))
        done
    done < <(echo "$dash" | jq -c '.panels[] | {title, rawSql: .targets[0].rawSql, format: .targets[0].format}')
done
# A deliberately broken query must be reported, or the loop above proves nothing.
bad=$(curl -s -H 'Content-Type: application/json' "$GRAFANA/api/ds/query" -d '{"queries":[{"refId":"A","datasource":{"type":"grafana-clickhouse-datasource","uid":"clickhouse"},"editorType":"sql","rawSql":"SELECT no_such_column FROM market.trade","format":1}],"from":"now-5m","to":"now"}')
[[ -n $(echo "$bad" | jq -r '.results.A.error // empty') ]] || fail "Grafana did not report an error for a bad query"
ok "$checked Grafana panel queries (per-table panels over all $(wc -w <<<"$tables" | tr -d ' ') tables) run without error and return rows"

# 4. Metrics, traces and Aeron's counters: recent, and attributed
apps=$( (tr A-Z a-z <<<"$expected" | tr , '\n'; "${KUBE[@]}" get deploy -l 'app in (engine,exch-sim)' -o jsonpath='{range .items[*]}{.metadata.name}{"\n"}{end}') | sort | paste -sd, -)
for table in metrics metrics_histogram otel_traces aeron_counters; do
    ts=$([[ $table == otel_traces ]] && echo Timestamp || echo ts)
    n=$(sql "SELECT count() FROM market.$table WHERE $ts > now() - INTERVAL 1 MINUTE AND host != '' AND pod != ''")
    (( n > 0 )) || fail "$table: no rows with a host and pod in the last minute"
done
got=$(sql "SELECT arrayStringConcat(arraySort(groupUniqArray(app)), ',') FROM market.metrics WHERE ts > now() - INTERVAL 1 MINUTE AND app != ''")
[[ $got == "$apps" ]] || fail "metrics in the last minute came from '$got', expected $apps"
ok "metrics, histograms, traces and Aeron counters arriving, with host and pod, from $got"

# 5. Notebook (before the toggle: it checks disabled tables got nothing for 30 s)
"${KUBE[@]}" exec deploy/jupyter -- jupyter nbconvert --to notebook --execute --stdout verify.ipynb >/dev/null \
    || fail "notebooks/verify.ipynb failed (open http://localhost:8888 to see where)"
ok "notebooks/verify.ipynb runs clean"

# 6. Live toggle of a dynamic table, through the cluster's ConfigMap: the
# kubelet takes up to about a minute to update it in the pods.
config=config/tables.yaml
saved=$(cat "$config")
push_config() { just config >/dev/null; }
restore() { printf "%s\n" "$saved" > "$config"; push_config; }
trap restore EXIT
set_book() { sed -E "s/^(  book_snapshot: \{ kind: dynamic, enabled: )(true|false)( \})/\1$1\3/" <<<"$saved" > "$config"; push_config; }
recent_books() { sql "SELECT count() FROM market.book_snapshot WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL $1 SECOND" 2>/dev/null || echo 0; }
set_book true
for _ in $(seq 150); do (( $(recent_books 5) > 0 )) && break; sleep 1; done
(( $(recent_books 5) > 0 )) || fail "book_snapshot enabled but no rows arrived within 150 s"
ok "book_snapshot on: rows arriving without a restart"
set_book false
stopped=
for _ in $(seq 25); do
    before=$(sql "SELECT count() FROM market.book_snapshot")
    sleep 6
    after=$(sql "SELECT count() FROM market.book_snapshot")
    [[ $before == "$after" ]] && { stopped=1; break; }
done
[[ -n $stopped ]] || fail "book_snapshot disabled but rows kept arriving 150 s later ($before -> $after)"
ok "book_snapshot off: row count stayed at $after"
restore
trap - EXIT
restarts=$("${KUBE[@]}" get pod -l 'app in (aeron,ingester,md,engine,exch-sim)' -o jsonpath='{range .items[*]}{.metadata.name}={.status.containerStatuses[0].restartCount} {end}')
[[ $restarts != *=[1-9]* ]] || fail "restarted: $restarts"
ok "Aeron, the ingesters, the feed handlers, the engines and the exchanges never restarted"

# 7. Move a feed handler to another node of its region
mover=md-binance other=HYPERLIQUID engine=engine-an1
if "${KUBE[@]}" get deploy $mover engine-an1 >/dev/null 2>&1; then
    old_pod=$("${KUBE[@]}" get pod -l app=md,exchange=binance -o jsonpath='{.items[0].metadata.name}')
    old_node=$("${KUBE[@]}" get pod "$old_pod" -o jsonpath='{.spec.nodeName}')
    metric() { sql "SELECT argMax(value, ts) FROM market.metrics WHERE app = '$engine' AND name = '$1' AND labels['venue'] = '$2'"; }
    resyncs=$(metric feed_resyncs BINANCE)
    started=$(sql "SELECT now64(9)")
    kubectl --context kind-clickhouse-lab cordon "$old_node" >/dev/null
    trap 'kubectl --context kind-clickhouse-lab uncordon "$old_node" >/dev/null' EXIT
    "${KUBE[@]}" delete pod "$old_pod" --wait=false >/dev/null
    for _ in $(seq 90); do
        new_pod=$("${KUBE[@]}" get pod -l app=md,exchange=binance -o jsonpath='{range .items[?(@.status.phase=="Running")]}{.metadata.name}{"\n"}{end}' | grep -v "^$old_pod\$" || true)
        [[ -n $new_pod ]] && break
        sleep 1
    done
    [[ -n $new_pod ]] || fail "$mover did not come back within 90 s"
    new_node=$("${KUBE[@]}" get pod "$new_pod" -o jsonpath='{.spec.nodeName}')
    [[ $new_node != "$old_node" ]] || fail "$mover came back on $old_node, the cordoned node"
    kubectl --context kind-clickhouse-lab uncordon "$old_node" >/dev/null
    trap - EXIT
    moved=$(date +%s)
    # Its engine joins the new node's publication, and resyncs its books.
    for _ in $(seq 60); do
        (( $(metric feed_resyncs BINANCE | cut -d. -f1) > ${resyncs%%.*} )) && break
        sleep 1
    done
    (( $(metric feed_resyncs BINANCE | cut -d. -f1) > ${resyncs%%.*} )) || fail "$engine did not resync BINANCE within 60 s of the move"
    for _ in $(seq 30); do
        age=$(sql "SELECT argMax(value, ts) FROM market.metrics WHERE app = '$engine' AND name = 'book_age_ns' AND labels['venue'] = 'BINANCE' AND ts > now() - INTERVAL 5 SECOND")
        [[ -n $age ]] && (( ${age%%.*} < 2000000000 )) && break
        sleep 1
    done
    [[ -n $age ]] && (( ${age%%.*} < 2000000000 )) || fail "$engine: BINANCE's book still stale after the move (${age:-no} ns)"
    recovered=$(( $(date +%s) - moved ))
    worst=$(sql "SELECT max(value) FROM market.metrics WHERE app = '$engine' AND name = 'book_age_ns' AND labels['venue'] = '$other' AND ts > '$started'")
    (( ${worst%%.*} < 5000000000 )) || fail "$engine: $other's book went ${worst} ns without an update during the move"
    # Recorded by the new node's archive, and nothing twice.
    for _ in $(seq 60); do
        hosts=$(sql "SELECT count() FROM market.trade WHERE venue = 'BINANCE' AND host = '$new_node' AND ts_event > '$started'")
        (( hosts > 0 )) && break
        sleep 1
    done
    (( hosts > 0 )) || fail "no BINANCE trades recorded from $new_node after the move"
    dupes=$(sql "SELECT count() - uniqExact(symbol, trade_id) FROM market.trade WHERE venue = 'BINANCE' AND ts_event > '$started' - INTERVAL 1 MINUTE")
    [[ $dupes == 0 ]] || fail "$dupes BINANCE trades recorded twice across the move"
    ok "$mover moved $old_node -> $new_node: $engine resynced it, its book fresh $recovered s after, $other never stale (worst $((${worst%%.*} / 1000000)) ms), trades from the new node, none twice"

    # Restarted in place: the same node's driver, so only a closed
    # publication makes the new pod a new session its engine can see.
    resyncs=$(metric feed_resyncs BINANCE)
    kubectl --context kind-clickhouse-lab cordon "$old_node" >/dev/null
    trap 'kubectl --context kind-clickhouse-lab uncordon "$old_node" >/dev/null' EXIT
    "${KUBE[@]}" delete pod "$new_pod" --wait=false >/dev/null
    for _ in $(seq 90); do
        again=$("${KUBE[@]}" get pod -l app=md,exchange=binance -o jsonpath='{range .items[?(@.status.phase=="Running")]}{.metadata.name}{"\n"}{end}' | grep -v "^$new_pod\$" || true)
        [[ -n $again ]] && break
        sleep 1
    done
    [[ -n $again ]] || fail "$mover did not come back within 90 s of its restart"
    [[ $("${KUBE[@]}" get pod "$again" -o jsonpath='{.spec.nodeName}') == "$new_node" ]] || fail "$mover's restart left $new_node"
    kubectl --context kind-clickhouse-lab uncordon "$old_node" >/dev/null
    trap - EXIT
    for _ in $(seq 60); do
        (( $(metric feed_resyncs BINANCE | cut -d. -f1) > ${resyncs%%.*} )) && break
        sleep 1
    done
    (( $(metric feed_resyncs BINANCE | cut -d. -f1) > ${resyncs%%.*} )) || fail "$engine saw no new session when $mover restarted on $new_node"
    ok "$mover restarted in place on $new_node: a new session, and $engine resynced it"
fi

echo "all checks passed"
