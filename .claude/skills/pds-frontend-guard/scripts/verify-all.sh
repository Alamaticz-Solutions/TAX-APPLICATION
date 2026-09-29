#!/usr/bin/env bash
# Full verification of the checked-out commit: everything that must pass before
# a push or a merge. Part of the pds-frontend-guard plugin (.claude/skills/pds-frontend-guard).
#
#   1. Preflight      tools and the database
#   2. Frontend gate  npm run test:frontend (scaffold, PHI, entities, PDS ratchet,
#                     typecheck, lint, unit + PDS rule tests, production build)
#   3. Backend        cargo check, backend + Rego tests, backend binary
#   4. Framework      appfw validate, generate --check, boundary-check, policy-test
#   5. Live           this build's backend against a database, the governance flow
#                     and audit hash-chain smoke tests, and the full Playwright E2E suite
#
# Two modes:
#   (default)  a developer machine. The database is the local `tax-doc-routing-postgres`
#              container; it is snapshotted before the live phase and restored after,
#              and a backend already running on the API port is stopped and restarted.
#              With a clean tree, HEAD is stamped in .git/pds-verify-passed so the
#              pre-push hook doesn't run everything again for the same commit.
#   --ci       Bitbucket Pipelines (bitbucket-pipelines.yml) or any Linux CI with a
#              Postgres service on localhost:5432 whose user/password match
#              backend/.env.example. It builds the database from scratch with the
#              framework's migrate, installs the E2E browser, and writes no stamp.
#
# Only the frontend has PDS rules and a guard for now. Locally this script checks a push that
# touches tax-doc-routing/frontend/ (compared with the remote branch), with no database, no
# Rust build and no live backend:
#   1. Preflight      node and git
#   2. Frontend gate  as above (PDS rule tests, ratchet, typecheck, lint, unit tests, build)
#   3. E2E            the Playwright suite (mocked API: layout, accessibility, PDS behaviour)
# and stamps the commit. A push with no frontend change has nothing to check here: it is stamped
# straight away and goes through as an ordinary push. The backend checks (steps 3-5 above) are
# kept for `--full` and `--ci`, for when the backend gets its own rules; they never run by default.
#   --frontend  force the frontend-only run     --full  run everything, including the backend
#
# Usage: bash .claude/skills/pds-frontend-guard/scripts/verify-all.sh [--ci] [--frontend|--full]
# Logs:  tax-doc-routing/target/verify/<step>.log

set -euo pipefail

CI_MODE=0
SCOPE=auto
for arg in "$@"; do
  case "$arg" in
    --ci) CI_MODE=1 ;;
    --frontend) SCOPE=frontend ;;
    --full) SCOPE=full ;;
    -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

ROOT="$(git rev-parse --show-toplevel)"
PG="$ROOT/tax-doc-routing"
FE="$PG/frontend"
LOGS="$PG/target/verify"
STAMP="$(git rev-parse --absolute-git-dir)/pds-verify-passed"
CONTAINER="tax-doc-routing-postgres"
DB="tax_routing"
E2E_PORT=5199
mkdir -p "$LOGS"

case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) WINDOWS=1; export MSYS_NO_PATHCONV=1 ;; *) WINDOWS=0 ;; esac
if [ "$CI_MODE" = 1 ]; then
  # A Bitbucket 2x step has 8 GB, 1 GB of it for the Postgres service. One rustc per CPU
  # with full debug info ran out of memory (SIGKILL) on the backend crate, so CI builds
  # with fewer parallel jobs and no debug info; what is checked and tested is unchanged.
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-0}" CARGO_INCREMENTAL=0
fi
EXE=""; [ "$WINDOWS" = 1 ] && EXE=".exe"
BACKEND_BIN="${CARGO_TARGET_DIR:-$PG/target}/debug/backend$EXE"
ENV_FILE="$PG/backend/.env"
PY=python3; "$PY" --version >/dev/null 2>&1 || PY=python

HEAD_AT_START="$(git rev-parse HEAD)"
CLEAN_AT_START=0; [ -z "$(git status --porcelain)" ] && CLEAN_AT_START=1
T0=$SECONDS
CHECKS=0

fail() { echo; echo "VERIFY FAILED: $*" >&2; exit 1; }

# The backend/live phase below was written for another product (governance schema, its smoke
# tests). Tax Document Routing has no smoke tests yet, so only the frontend scope is supported.
# Remove this guard once tax-doc-routing/scripts/smoke/ exists and the phase is adapted.
if [ "$CI_MODE" = 1 ] || [ "$SCOPE" = full ]; then
  fail "--full / --ci are not supported yet for Tax Document Routing (no live smoke tests); use --frontend"
fi

# run <name> <command...>: logs to $LOGS/<name>.log, prints the tail on failure.
run() {
  local name="$1"; shift
  local log="$LOGS/$name.log" t=$SECONDS
  printf '  %-26s' "$name"
  if "$@" >"$log" 2>&1; then
    printf 'ok   (%ss)\n' "$((SECONDS - t))"
    CHECKS=$((CHECKS + 1))
  else
    printf 'FAIL (%ss)\n' "$((SECONDS - t))"
    echo "---- last 40 lines of $log ----"; tail -40 "$log" | sed 's/\x1b\[[0-9;]*m//g'
    fail "$name (full log: $log)"
  fi
}

env_value() { sed -n "s/^$1=//p" "$ENV_FILE" 2>/dev/null | head -1 | tr -d '\r'; }

# The framework CLI: in the rust-appfw container on Windows, natively elsewhere.
appfw() {
  if [ "$WINDOWS" = 1 ]; then
    docker run --rm -v "$ROOT":/work -w /work/tax-doc-routing rust-appfw:latest bash scripts/appfw "$@"
  else
    (cd "$PG" && bash scripts/appfw "$@")
  fi
}
# An appfw check passes when it exits 0 and its JSON report says "ok": true.
appfw_ok() {
  local out
  out="$(appfw "$@")" || { printf '%s\n' "$out"; return 1; }
  printf '%s\n' "$out"
  case "$out" in *'"ok": true'*|*'"ok":true'*) return 0 ;; *) return 1 ;; esac
}

# psql against the database: the local container, or the CI service on localhost.
psql_c() {
  if [ "$CI_MODE" = 1 ]; then
    PGPASSWORD="$PGPASS" psql -h "${PGHOST:-localhost}" -p "${PGPORT:-5432}" -U "$PGSUPER" -d "$1" -v ON_ERROR_STOP=1 -Atqc "$2"
  else
    docker exec "$CONTAINER" psql -U "$PGSUPER" -d "$1" -Atqc "$2"
  fi
}
counts() { psql_c "$DB" "select (select count(*) from governance.projects)||'/'||(select count(*) from governance.users)||'/'||(select count(*) from governance.audit_events)"; }

# OS process ids listening on a TCP port (Windows pids on Windows).
port_pids() {
  if [ "$WINDOWS" = 1 ]; then
    netstat -ano | awk -v p=":$1" '$1=="TCP" && $2 ~ p"$" && $4=="LISTENING" {print $5}' | sort -u
  else
    (ss -ltnp 2>/dev/null | awk -v p=":$1" '$4 ~ p"$"' | grep -o 'pid=[0-9]*' | cut -d= -f2 | sort -u) || true
  fi
}
pid_path() {
  if [ "$WINDOWS" = 1 ]; then
    powershell.exe -NoProfile -Command "(Get-Process -Id $1 -ErrorAction SilentlyContinue).Path" | tr -d '\r'
  else
    readlink -f "/proc/$1/exe" 2>/dev/null || true
  fi
}
# Single-slash switches: MSYS_NO_PATHCONV=1 (set above for Docker paths) stops
# Git Bash turning //PID into /PID.
kill_pid() { if [ "$WINDOWS" = 1 ]; then taskkill /PID "$1" /T /F >/dev/null 2>&1 || true; else kill "$1" 2>/dev/null || true; fi; }
kill_port() { local p; for p in $(port_pids "$1"); do kill_pid "$p"; done; wait_port_free "$1"; }
wait_port_free() { local i; for ((i = 0; i < 15; i++)); do [ -z "$(port_pids "$1")" ] && return 0; sleep 1; done; return 1; }
# Output goes through shell redirection, not curl -o: with MSYS_NO_PATHCONV=1, Windows curl
# would receive "/dev/null" literally and fail on every successful response.
wait_http() { local i; for ((i = 0; i < $2; i++)); do curl -sf "$1" >/dev/null 2>&1 && return 0; sleep 1; done; return 1; }
# Detach a long-running server; nohup only where SIGHUP exists (not Git Bash).
detach() { if [ "$WINDOWS" = 1 ]; then "$@"; else nohup "$@"; fi; }
start_backend() { (cd "$PG/backend" && detach "$BACKEND_BIN" >"$1" 2>&1 </dev/null &); }

restore_db() {
  psql_c postgres "DROP DATABASE IF EXISTS $DB WITH (FORCE)" \
    && psql_c postgres "CREATE DATABASE $DB OWNER $PGSUPER" \
    && docker exec -i "$CONTAINER" pg_restore -U "$PGSUPER" -d "$DB" --exit-on-error <"$SNAPSHOT" || return 1
  local now; now="$(counts)"
  if [ "$now" = "$SNAP_COUNTS" ]; then
    rm -f "$SNAPSHOT"; echo "  ok   database restored (projects/users/audit events: $now)"
  else
    echo "  WARNING: restored counts $now differ from $SNAP_COUNTS; snapshot kept at $SNAPSHOT" >&2; return 1
  fi
}

# CI: build the database the framework way (docs/LOCAL_DEV_SETUP.md §6).
migrate_ci() {
  local seed="$LOGS/seed.patched.sql"
  # Tables and schema. The generated seed then fails (PDS Finding Q), so this
  # step's own result is not the check; the patched seed and apply below are.
  (cd "$PG" && ENV_NAME=local PG_SERVICE_ACCOUNT_NAME="$PGSUPER" PG_SERVICE_ACCOUNT_PASS="$PGPASS" bash scripts/appfw product migrate) || true
  psql_c "$DB" "select to_regclass('governance.projects')" | grep -q projects || { echo "schema was not created"; return 1; }
  sed "s/ARRAY\[\]::varchar\[\]/'[]'::jsonb/g; s/ARRAY\['admin'\]/'[\"admin\"]'::jsonb/g" \
    "$PG/database/_pkg/schemas/governance/seed.pg.sql" >"$seed"
  { echo "SET session_replication_role = replica;"; cat "$seed"; } \
    | PGPASSWORD="$PGPASS" psql -h "${PGHOST:-localhost}" -p "${PGPORT:-5432}" -U "$PGSUPER" -d "$DB" -v ON_ERROR_STOP=1 -q
  (cd "$PG" && ENV_NAME=local PG_SERVICE_ACCOUNT_NAME="$PGSUPER" PG_SERVICE_ACCOUNT_PASS="$PGPASS" \
    bash scripts/appfw product migrate -- apply --phase all --confirm-contract)
  echo "database: $(counts) projects/users/audit events"
}

# ---- live-phase state; cleanup runs on success, failure and Ctrl-C ----
SNAPSHOT=""; SNAP_COUNTS=""; RESTORED=0; USER_BACKEND_WAS_RUNNING=0; E2E_SERVER=0; CI_BACKEND=0
cleanup() {
  local code=$?
  set +e
  [ "$E2E_SERVER" = 1 ] && kill_port "$E2E_PORT"
  [ "$CI_BACKEND" = 1 ] && kill_port "$PORT"
  if [ -n "$SNAPSHOT" ] && [ "$RESTORED" = 0 ]; then
    echo "  restoring the database from $SNAPSHOT"
    kill_port "$PORT"
    restore_db && RESTORED=1
  fi
  if [ "$USER_BACKEND_WAS_RUNNING" = 1 ] && [ -z "$(port_pids "$PORT")" ]; then
    start_backend "$LOGS/backend-restarted.log"
    echo "  your backend is running again on :$PORT (this build; log $LOGS/backend-restarted.log)"
  fi
  exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

# Does this push touch the frontend? Compare HEAD with the remote branch (or main when it has no
# remote yet). When the base can't be found, assume it does: check rather than skip.
push_touches_frontend() {
  local base
  base="$(git rev-parse --verify -q '@{upstream}' 2>/dev/null || git merge-base origin/main HEAD 2>/dev/null || true)"
  [ -n "$base" ] || return 0
  git diff --name-only "$base" HEAD | grep -q '^tax-doc-routing/frontend/'
}
FRONTEND_ONLY=0
if [ "$CI_MODE" = 0 ]; then
  case "$SCOPE" in
    frontend) FRONTEND_ONLY=1 ;;
    full) ;;
    auto)
      if push_touches_frontend; then
        FRONTEND_ONLY=1
      else
        STAMPED="no stamp (the working tree had changes)"
        if [ "$CLEAN_AT_START" = 1 ]; then echo "$HEAD_AT_START" >"$STAMP"; STAMPED="$(git rev-parse --short HEAD) recorded as verified"; fi
        echo "Verifying $(git rev-parse --short HEAD): no frontend changes in this push, so there are no PDS checks to run; $STAMPED"
        exit 0
      fi ;;
  esac
fi

if [ "$FRONTEND_ONLY" = 1 ]; then
  echo "Verifying $(git rev-parse --short HEAD) (frontend only: no database, no Rust build)$([ "$CLEAN_AT_START" = 1 ] || echo ' — working tree has changes: no stamp will be written')"
  echo "1. Preflight"
  node -e 'process.exit(+process.versions.node.split(".")[0] >= 20 ? 0 : 1)' || fail "Node 20 or newer is required (package.json engines)"
  command -v git >/dev/null || fail "git not found"
  [ -z "$(port_pids "$E2E_PORT")" ] || fail "port $E2E_PORT (the E2E dev server) is already in use"
  echo "  ok   node $(node -v)"
  echo "2. Frontend gate"
  if [ ! -d "$FE/node_modules" ] || [ "$FE/package-lock.json" -nt "$FE/node_modules/.package-lock.json" ]; then
    run npm-ci bash -c "cd \"$FE\" && npm ci"
  fi
  run frontend-gate bash -c "cd \"$FE\" && npm run test:frontend"
  echo "3. E2E (mocked API)"
  # Tax Document Routing has no Playwright specs yet (no tax-doc-routing/frontend/tests/ dir),
  # unlike the product this script was adapted from. Same reasoning as the backend live-phase
  # guard above: skip rather than fail on an empty suite, and remove this once specs exist.
  if [ ! -d "$FE/tests" ] || ! find "$FE/tests" -name '*.spec.ts' -print -quit | grep -q .; then
    echo "  skip (no Playwright specs under $FE/tests yet)"
  else
    E2E_SERVER=1
    (cd "$FE" && detach npx vite --port "$E2E_PORT" --strictPort >"$LOGS/vite.log" 2>&1 </dev/null &)
    wait_http "http://localhost:$E2E_PORT/" 120 || fail "the E2E dev server did not start (log $LOGS/vite.log)"
    curl -s --max-time 180 "http://localhost:$E2E_PORT/src/main.tsx" >/dev/null 2>&1 || true
    run e2e bash -c "cd \"$FE\" && env -u CI npx playwright test --reporter=line"
    kill_port "$E2E_PORT"; E2E_SERVER=0
  fi
  STAMPED="no stamp (the working tree had changes or HEAD moved)"
  if [ "$CLEAN_AT_START" = 1 ] && [ "$(git rev-parse HEAD)" = "$HEAD_AT_START" ] && [ -z "$(git status --porcelain)" ]; then
    echo "$HEAD_AT_START" >"$STAMP"
    STAMPED="$(git rev-parse --short HEAD) recorded as verified"
  fi
  echo
  echo "VERIFY PASSED (frontend only) in $(((SECONDS - T0) / 60))m $(((SECONDS - T0) % 60))s: $CHECKS checks; $STAMPED"
  exit 0
fi

MODE="local"; [ "$CI_MODE" = 1 ] && MODE="CI"
echo "Verifying $(git rev-parse --short HEAD) ($MODE)$([ "$CI_MODE" = 1 ] || [ "$CLEAN_AT_START" = 1 ] || echo ' — working tree has changes: no stamp will be written')"

echo "1. Preflight"
node -e 'process.exit(+process.versions.node.split(".")[0] >= 20 ? 0 : 1)' || fail "Node 20 or newer is required (package.json engines)"
for tool in cargo curl git; do command -v "$tool" >/dev/null || fail "$tool not found"; done
[ "$WINDOWS" = 1 ] || command -v rsync >/dev/null || fail "rsync not found (the framework CLI needs it)"
if [ "$CI_MODE" = 1 ]; then
  [ -f "$ENV_FILE" ] || cp "$PG/backend/.env.example" "$ENV_FILE"
  PGSUPER="$(env_value PG_SERVICE_ACCOUNT_NAME)"; PGPASS="$(env_value PG_SERVICE_ACCOUNT_PASS)"
  command -v psql >/dev/null || fail "psql not found"
  for ((i = 0; i < 60; i++)); do psql_c postgres "select 1" >/dev/null 2>&1 && break; sleep 1; done
  psql_c postgres "select 1" >/dev/null 2>&1 || fail "cannot reach Postgres at ${PGHOST:-localhost}:${PGPORT:-5432} as $PGSUPER"
  psql_c postgres "select 1 from pg_database where datname = '$DB'" | grep -q 1 || psql_c postgres "CREATE DATABASE $DB OWNER $PGSUPER"
else
  [ -f "$ENV_FILE" ] || fail "backend/.env is missing (copy backend/.env.example; docs/LOCAL_DEV_SETUP.md §4)"
  docker info >/dev/null 2>&1 || fail "Docker is not running"
  [ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER" 2>/dev/null)" = true ] || fail "container $CONTAINER is not running: docker start $CONTAINER"
  PGSUPER="$(docker exec "$CONTAINER" printenv POSTGRES_USER)"
  [ "$WINDOWS" = 0 ] || docker image inspect rust-appfw:latest >/dev/null 2>&1 || fail "image rust-appfw:latest missing (docs/LOCAL_DEV_SETUP.md)"
fi
PORT="$(env_value API_PORT)"; PORT="${PORT:-8080}"
[ -z "$(port_pids "$E2E_PORT")" ] || fail "port $E2E_PORT (the E2E dev server) is already in use"
echo "  ok   node $(node -v), db user $PGSUPER, API port $PORT"

echo "2. Frontend gate"
if [ ! -d "$FE/node_modules" ] || [ "$FE/package-lock.json" -nt "$FE/node_modules/.package-lock.json" ]; then
  run npm-ci bash -c "cd \"$FE\" && npm ci"
fi
run frontend-gate bash -c "cd \"$FE\" && npm run test:frontend"

echo "3. Backend"
run cargo-check bash -c "cd \"$PG\" && cargo check --workspace --all-targets"
run backend-tests bash -c "cd \"$PG\" && cargo test -p backend"
run rego-tests bash -c "cd \"$PG\" && cargo test -p rego_test"
run backend-build bash -c "cd \"$PG\" && cargo build -p backend --bin backend"

echo "4. Framework checks"
run appfw-validate appfw_ok product validate --json
run appfw-drift appfw_ok product generate --check --json
run appfw-boundary appfw_ok product boundary-check --json
run appfw-policy appfw_ok product policy-test --json

if [ "$CI_MODE" = 1 ]; then
  echo "5. Live (fresh database -> this build)"
  run db-migrate migrate_ci
  run e2e-browser bash -c "cd \"$FE\" && npx playwright install --with-deps chromium"
  export GOV_PSQL="psql -h ${PGHOST:-localhost} -p ${PGPORT:-5432} -U $PGSUPER -d $DB"
  export PGPASSWORD="$PGPASS"
  CI_BACKEND=1
else
  echo "5. Live (database snapshot -> this build -> restore)"
  for pid in $(port_pids "$PORT"); do
    path="$(pid_path "$pid")"
    case "$path" in
      *[/\\]tax-doc-routing[/\\]target[/\\]debug[/\\]backend*) USER_BACKEND_WAS_RUNNING=1; kill_pid "$pid" ;;
      *) fail "port $PORT is used by another program (${path:-pid $pid}); stop it and retry" ;;
    esac
  done
  wait_port_free "$PORT" || fail "could not stop the backend on port $PORT"
  mkdir -p "$ROOT/.db-backups"
  SNAP_COUNTS="$(counts)"
  SNAPSHOT="$ROOT/.db-backups/verify-$(date +%Y%m%d-%H%M%S).dump"
  docker exec "$CONTAINER" pg_dump -U "$PGSUPER" -d "$DB" -Fc >"$SNAPSHOT" || fail "database snapshot failed"
  echo "  ok   snapshot taken (projects/users/audit events: $SNAP_COUNTS)"
fi

start_backend "$LOGS/backend.log"
wait_http "http://127.0.0.1:$PORT/health/ready" 120 || { tail -20 "$LOGS/backend.log"; fail "backend did not become ready"; }
run live-governance-flow bash -c "cd \"$PG\" && PYTHONIOENCODING=utf8 $PY scripts/smoke/governance_flow.py"
run live-audit-smoke bash -c "cd \"$PG\" && PYTHONIOENCODING=utf8 $PY scripts/smoke/smoke_test.py"

# Playwright reuses a dev server already on $E2E_PORT (with CI unset). Start and
# warm it first, so a cold dependency pre-bundle can't time out the first specs.
E2E_SERVER=1
(cd "$FE" && detach npx vite --port "$E2E_PORT" --strictPort >"$LOGS/vite.log" 2>&1 </dev/null &)
wait_http "http://localhost:$E2E_PORT/" 120 || fail "the E2E dev server did not start (log $LOGS/vite.log)"
curl -s --max-time 180 "http://localhost:$E2E_PORT/src/main.tsx" >/dev/null 2>&1 || true
run e2e bash -c "cd \"$FE\" && env -u CI npx playwright test --reporter=line"
kill_port "$E2E_PORT"; E2E_SERVER=0
kill_port "$PORT"; CI_BACKEND=0

STAMPED="CI run (no stamp)"
if [ "$CI_MODE" = 0 ]; then
  restore_db || fail "database restore did not match the snapshot"
  RESTORED=1
  if [ "$CLEAN_AT_START" = 1 ] && [ "$(git rev-parse HEAD)" = "$HEAD_AT_START" ] && [ -z "$(git status --porcelain)" ]; then
    echo "$HEAD_AT_START" >"$STAMP"
    STAMPED="$(git rev-parse --short HEAD) recorded as verified"
  else
    STAMPED="no stamp (the working tree had changes or HEAD moved)"
  fi
fi
echo
echo "VERIFY PASSED in $(((SECONDS - T0) / 60))m $(((SECONDS - T0) % 60))s: $CHECKS checks; $STAMPED"
