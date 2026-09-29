#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
load-test-generated-api - lightweight generated GraphQL API load harness

Usage:
  scripts/load-test-generated-api.sh [options]

Options:
  --json                 Emit a JSON summary
  --url URL              GraphQL endpoint URL
  --requests N           Total requests to send
  --concurrency N        Requests to run per batch
  --body-file PATH       JSON request body to POST
  --header NAME:VALUE    Extra header; can be repeated
  --scenario NAME        Scenario name recorded in evidence
  --max-p95-ms N         Fail when p95 latency exceeds N milliseconds
  --max-max-ms N         Fail when max latency exceeds N milliseconds
  --max-error-rate N     Fail when failed / total exceeds N (for example 0.01)
  --output PATH          Write JSON evidence to PATH
  --help                 Show this help

Environment:
  APPFW_LOAD_TEST_URL
  APPFW_LOAD_TEST_REQUESTS
  APPFW_LOAD_TEST_CONCURRENCY
  APPFW_LOAD_TEST_BODY
  APPFW_LOAD_TEST_BODY_FILE
  APPFW_LOAD_TEST_TOKEN
  APPFW_LOAD_TEST_TIMEZONE
  APPFW_LOAD_TEST_SCENARIO
  APPFW_LOAD_TEST_MAX_P95_MS
  APPFW_LOAD_TEST_MAX_MAX_MS
  APPFW_LOAD_TEST_MAX_ERROR_RATE
  APPFW_LOAD_TEST_OUTPUT

The default body targets the sample CRM model:
  queryAccounts(skip: 0, limit: 10) { items { id name } }
EOF
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '"%s"' "$value"
}

json_summary() {
  local ok="$1"
  local detail="${2:-}"
  printf '{"command":"load-test","ok":%s' "$ok"
  if [[ -n "$detail" ]]; then
    printf ',"detail":'
    json_escape "$detail"
  fi
  printf '}\n'
}

usage_error() {
  local message="$1"
  if [[ "${json:-0}" == "1" ]]; then
    json_summary false "$message"
  else
    echo "$message" >&2
  fi
  exit 2
}

json=0
url="${APPFW_LOAD_TEST_URL:-http://127.0.0.1:8080/crm}"
requests="${APPFW_LOAD_TEST_REQUESTS:-100}"
concurrency="${APPFW_LOAD_TEST_CONCURRENCY:-8}"
body_file="${APPFW_LOAD_TEST_BODY_FILE:-}"
body="${APPFW_LOAD_TEST_BODY:-}"
timezone="${APPFW_LOAD_TEST_TIMEZONE:-America/Denver}"
token="${APPFW_LOAD_TEST_TOKEN:-}"
scenario="${APPFW_LOAD_TEST_SCENARIO:-generated-api-query}"
max_p95_ms="${APPFW_LOAD_TEST_MAX_P95_MS:-}"
max_max_ms="${APPFW_LOAD_TEST_MAX_MAX_MS:-}"
max_error_rate="${APPFW_LOAD_TEST_MAX_ERROR_RATE:-0}"
output_file="${APPFW_LOAD_TEST_OUTPUT:-}"
headers=("")

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      json=1
      ;;
    --url)
      shift || true
      url="${1:-}"
      ;;
    --requests)
      shift || true
      requests="${1:-}"
      ;;
    --concurrency)
      shift || true
      concurrency="${1:-}"
      ;;
    --body-file)
      shift || true
      body_file="${1:-}"
      ;;
    --header)
      shift || true
      headers+=("${1:-}")
      ;;
    --scenario)
      shift || true
      scenario="${1:-}"
      ;;
    --max-p95-ms)
      shift || true
      max_p95_ms="${1:-}"
      ;;
    --max-max-ms)
      shift || true
      max_max_ms="${1:-}"
      ;;
    --max-error-rate)
      shift || true
      max_error_rate="${1:-}"
      ;;
    --output)
      shift || true
      output_file="${1:-}"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      usage_error "unknown load-test option: $1"
      ;;
  esac
  shift || true
done

if ! command -v curl >/dev/null 2>&1; then
  if [[ "$json" == "1" ]]; then
    json_summary false "curl is required"
  else
    echo "curl is required" >&2
  fi
  exit 2
fi

case "$requests" in
  ''|*[!0-9]*)
    usage_error "--requests must be a positive integer"
    ;;
esac
case "$concurrency" in
  ''|*[!0-9]*)
    usage_error "--concurrency must be a positive integer"
    ;;
esac
if [[ "$requests" -le 0 || "$concurrency" -le 0 ]]; then
  usage_error "--requests and --concurrency must be greater than 0"
fi
for numeric_value in "$max_p95_ms" "$max_max_ms" "$max_error_rate"; do
  if [[ -n "$numeric_value" && ! "$numeric_value" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    usage_error "load-test thresholds must be non-negative numbers"
  fi
done

if [[ -n "$body_file" ]]; then
  body="$(cat "$body_file")"
fi
if [[ -z "$body" ]]; then
  body='{"query":"query LoadTest($skip:Int!, $limit:Int!){ queryAccounts(skip:$skip, limit:$limit){ query_count items { id name } } }","variables":{"skip":0,"limit":10}}'
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/appfw-load-test.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

run_request() {
  local index="$1"
  local output_file="$tmp_dir/$index.result"
  local curl_args=(
    -sS
    -o /dev/null
    -w "%{http_code} %{time_total}"
    -X POST
    -H "content-type: application/json"
    -H "x-timezone: $timezone"
  )
  if [[ -n "$token" ]]; then
    curl_args+=(-H "authorization: Bearer $token")
  fi
  local header
  for header in "${headers[@]}"; do
    if [[ -z "$header" ]]; then
      continue
    fi
    curl_args+=(-H "$header")
  done
  curl_args+=(--data "$body" "$url")

  local result rc
  set +e
  result="$(curl "${curl_args[@]}" 2>"$tmp_dir/$index.err")"
  rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    printf '%s 000 0\n' "$rc" >"$output_file"
    return
  fi
  printf '0 %s\n' "$result" >"$output_file"
}

started_epoch="$(date +%s)"
batch_count=0
i=1
while [[ "$i" -le "$requests" ]]; do
  run_request "$i" &
  batch_count=$((batch_count + 1))
  if [[ "$batch_count" -ge "$concurrency" ]]; then
    wait
    batch_count=0
  fi
  i=$((i + 1))
done
wait
ended_epoch="$(date +%s)"

summary="$(
  awk '
    {
      total += 1;
      rc = $1;
      status = $2;
      seconds = $3 + 0;
      ms = seconds * 1000;
      if (total == 1 || ms < min_ms) min_ms = ms;
      if (ms > max_ms) max_ms = ms;
      sum_ms += ms;
      if (rc == 0 && status >= 200 && status < 300) success += 1;
      else failed += 1;
      times[total] = ms;
    }
    END {
      for (i = 1; i <= total; i++) {
        for (j = i + 1; j <= total; j++) {
          if (times[j] < times[i]) {
            tmp = times[i]; times[i] = times[j]; times[j] = tmp;
          }
        }
      }
      if (total > 0) {
        avg_ms = sum_ms / total;
        p95_index = int(total * 0.95);
        if (p95_index < 1) p95_index = 1;
        if (p95_index > total) p95_index = total;
        p95_ms = times[p95_index];
      }
      printf "%d %d %d %.3f %.3f %.3f %.3f\n", total, success, failed, min_ms, avg_ms, p95_ms, max_ms;
    }
  ' "$tmp_dir"/*.result
)"

read -r total success failed min_ms avg_ms p95_ms max_ms <<<"$summary"
duration_seconds=$((ended_epoch - started_epoch))
error_rate="$(
  awk -v failed="$failed" -v total="$total" 'BEGIN {
    if (total > 0) printf "%.6f", failed / total;
    else printf "1.000000";
  }'
)"
ok=true
exit_code=0
if [[ "$failed" -ne 0 ]]; then
  ok=false
  exit_code=1
fi
float_gt() {
  awk -v left="$1" -v right="$2" 'BEGIN { exit !(left > right) }'
}
violations=("")
has_violations=0
if [[ -n "$max_p95_ms" ]] && float_gt "$p95_ms" "$max_p95_ms"; then
  ok=false
  exit_code=1
  violations+=("p95 latency ${p95_ms}ms exceeds threshold ${max_p95_ms}ms")
  has_violations=1
fi
if [[ -n "$max_max_ms" ]] && float_gt "$max_ms" "$max_max_ms"; then
  ok=false
  exit_code=1
  violations+=("max latency ${max_ms}ms exceeds threshold ${max_max_ms}ms")
  has_violations=1
fi
if [[ -n "$max_error_rate" ]] && float_gt "$error_rate" "$max_error_rate"; then
  ok=false
  exit_code=1
  violations+=("error rate ${error_rate} exceeds threshold ${max_error_rate}")
  has_violations=1
fi

if [[ "$json" == "1" ]]; then
  evidence_file="$tmp_dir/load-test.json"
  printf '{"command":"load-test","ok":%s,"scenario":' "$ok" >"$evidence_file"
  json_escape "$scenario" >>"$evidence_file"
  printf ',"url":' >>"$evidence_file"
  json_escape "$url" >>"$evidence_file"
  printf ',"requests":%s,"concurrency":%s,"success":%s,"failed":%s,"error_rate":%s,"duration_seconds":%s' \
    "$total" "$concurrency" "$success" "$failed" "$error_rate" "$duration_seconds" >>"$evidence_file"
  printf ',"latency_ms":{"min":%s,"avg":%s,"p95":%s,"max":%s}' \
    "$min_ms" "$avg_ms" "$p95_ms" "$max_ms" >>"$evidence_file"
  printf ',"thresholds":{"max_error_rate":%s' "$max_error_rate" >>"$evidence_file"
  if [[ -n "$max_p95_ms" ]]; then
    printf ',"max_p95_ms":%s' "$max_p95_ms" >>"$evidence_file"
  else
    printf ',"max_p95_ms":null' >>"$evidence_file"
  fi
  if [[ -n "$max_max_ms" ]]; then
    printf ',"max_max_ms":%s' "$max_max_ms" >>"$evidence_file"
  else
    printf ',"max_max_ms":null' >>"$evidence_file"
  fi
  printf '},"violations":[' >>"$evidence_file"
  first_violation=1
  for violation in "${violations[@]}"; do
    if [[ -z "$violation" ]]; then
      continue
    fi
    if [[ "$first_violation" != "1" ]]; then
      printf ',' >>"$evidence_file"
    fi
    json_escape "$violation" >>"$evidence_file"
    first_violation=0
  done
  printf ']}\n' >>"$evidence_file"
  if [[ -n "$output_file" ]]; then
    mkdir -p "$(dirname "$output_file")"
    cp "$evidence_file" "$output_file"
  fi
  cat "$evidence_file"
else
  echo "Load test: $url"
  echo "  scenario:    $scenario"
  echo "  requests:    $total"
  echo "  concurrency: $concurrency"
  echo "  success:     $success"
  echo "  failed:      $failed"
  echo "  error rate:  $error_rate"
  echo "  duration:    ${duration_seconds}s"
  echo "  latency ms:  min=$min_ms avg=$avg_ms p95=$p95_ms max=$max_ms"
  if [[ "$has_violations" == "1" ]]; then
    echo "  violations:"
    for violation in "${violations[@]}"; do
      if [[ -z "$violation" ]]; then
        continue
      fi
      echo "    - $violation"
    done
  fi
fi

exit "$exit_code"
