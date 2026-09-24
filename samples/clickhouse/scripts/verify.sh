#!/usr/bin/env bash
# End-to-end checks against the running lab (`just up` first).
#
#  1. ClickHouse answers, and its /play UI is served.
#  2. The recorder is writing trades and quotes from both venues.
#  3. Every Grafana panel's query runs through Grafana without error.
#  4. The verification notebook runs clean inside JupyterLab's pod.
#  5. Toggling a dynamic table in config/tables.yaml starts and stops it live.
#
# Leaves config/tables.yaml exactly as it found it.
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

# 2. Live data from both venues
venues=$(sql "SELECT arrayStringConcat(arraySort(groupUniqArray(venue)), ',') FROM market.trade WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL 1 MINUTE")
[[ $venues == "BINANCE,BYBIT" ]] || fail "trades in the last minute came from '$venues', expected BINANCE,BYBIT"
quotes=$(sql "SELECT count() FROM market.quote WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL 1 MINUTE")
(( quotes > 0 )) || fail "no quotes in the last minute"
ok "trades from $venues and $quotes quotes in the last minute"

# 3. Grafana panels
uids=$(curl -sf "$GRAFANA/api/search?type=dash-db" | jq -r '.[].uid')
[[ -n $uids ]] || fail "Grafana has no dashboards"
checked=0
tables=$(sql "SELECT name FROM system.tables WHERE database = 'market' ORDER BY name")
disabled() { grep -Eq "^  $1: .*enabled: false" config/tables.yaml; }
for uid in $uids; do
    dash=$(curl -sf "$GRAFANA/api/dashboards/uid/$uid" | jq '.dashboard')
    adhoc=$(echo "$dash" | jq -r '.templating.list[]? | select(.name=="sql") | .query // empty')
    while IFS= read -r target; do
        title=$(echo "$target" | jq -r '.title')
        raw=$(echo "$target" | jq -r '.rawSql')
        raw=${raw//'${sql:raw}'/$adhoc}
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
            # A disabled table may legitimately have nothing recent.
            [[ $rows -gt 0 ]] || disabled "$table" || { [[ $title == *book_snapshot* ]] && disabled book_snapshot; } \
                || fail "Grafana panel '$title' [$table] ($uid) returned no rows"
            checked=$((checked + 1))
        done
    done < <(echo "$dash" | jq -c '.panels[] | {title, rawSql: .targets[0].rawSql, format: .targets[0].format}')
done
# A deliberately broken query must be reported, or the loop above proves nothing.
bad=$(curl -s -H 'Content-Type: application/json' "$GRAFANA/api/ds/query" -d '{"queries":[{"refId":"A","datasource":{"type":"grafana-clickhouse-datasource","uid":"clickhouse"},"editorType":"sql","rawSql":"SELECT no_such_column FROM market.trade","format":1}],"from":"now-5m","to":"now"}')
[[ -n $(echo "$bad" | jq -r '.results.A.error // empty') ]] || fail "Grafana did not report an error for a bad query"
ok "$checked Grafana panel queries (per-table panels over all $(wc -w <<<"$tables" | tr -d ' ') tables) run without error and return rows"

# 4. Notebook (before the toggle: it checks disabled tables got nothing for 30 s)
"${KUBE[@]}" exec deploy/jupyter -- jupyter nbconvert --to notebook --execute --stdout verify.ipynb >/dev/null \
    || fail "notebooks/verify.ipynb failed (open http://localhost:8888 to see where)"
ok "notebooks/verify.ipynb runs clean"

# 5. Live toggle of a dynamic table
config=config/tables.yaml
saved=$(cat "$config")
trap 'printf "%s\n" "$saved" > "$config"' EXIT
set_book() { sed -E "s/^(  book_snapshot: \{ kind: dynamic, enabled: )(true|false)( \})/\1$1\3/" <<<"$saved" > "$config"; }
recent_books() { sql "SELECT count() FROM market.book_snapshot WHERE ts_event > now() - INTERVAL 10 MINUTE AND inserted_at > now() - INTERVAL $1 SECOND" 2>/dev/null || echo 0; }
set_book true
for _ in $(seq 30); do (( $(recent_books 5) > 0 )) && break; sleep 1; done
(( $(recent_books 5) > 0 )) || fail "book_snapshot enabled but no rows arrived within 30 s"
ok "book_snapshot on: rows arriving without a restart"
set_book false
sleep 3
before=$(sql "SELECT count() FROM market.book_snapshot")
sleep 5
after=$(sql "SELECT count() FROM market.book_snapshot")
[[ $before == "$after" ]] || fail "book_snapshot disabled but rows kept arriving ($before -> $after)"
ok "book_snapshot off: row count stayed at $after"
printf "%s\n" "$saved" > "$config"
trap - EXIT
restarts=$("${KUBE[@]}" get pod -l app=recorder -o jsonpath='{.items[0].status.containerStatuses[0].restartCount}')
[[ $restarts == 0 ]] || fail "recorder restarted $restarts times"
ok "recorder never restarted"

echo "all checks passed"
