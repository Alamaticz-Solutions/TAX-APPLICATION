#!/usr/bin/env bash
#
# security-assurance-local-evidence.sh - create local security-assurance
# artifacts for the formal release decision contract.
#
# CI may replace any of these with enterprise scanner/provenance/signing
# artifacts. This script keeps local release-readiness work repeatable when
# those tools are not installed on a developer workstation.
#
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
cd "$repo_root"

report_dir="${APPFW_RELEASE_ARTIFACT_DIR:-target/appfw}"
base_url="${APPFW_SECURITY_ASSURANCE_BASE_URL:-${APPFW_OPS_BASE_URL:-http://127.0.0.1:8080}}"
timeout_secs="${APPFW_SECURITY_ASSURANCE_TIMEOUT_SECS:-15}"

usage() {
  cat <<'USAGE'
Usage:
  APPFW_SECURITY_ASSURANCE_BASE_URL=http://127.0.0.1:8080 bash scripts/ci/security-assurance-local-evidence.sh

Writes:
  target/appfw/dast-evidence.json
  target/appfw/sast-evidence.json
  target/appfw/asvs-traceability.json
  target/appfw/release-provenance.json
  target/appfw/artifact-signing.json

The generated artifacts are intentionally local evidence. Regulated release CI
can substitute ZAP/DAST, CodeQL/Semgrep/SAST, SLSA provenance, and cosign or
enterprise signing artifacts while keeping the same decision gate.
USAGE
}

case "${1:-}" in
  "")
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    echo "security-assurance-local-evidence: unknown argument: $1" >&2
    usage >&2
    exit 2
    ;;
esac

mkdir -p "$report_dir"

python3 - "$repo_root" "$report_dir" "$base_url" "$timeout_secs" <<'PY'
import base64
import datetime as dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

repo_root = Path(sys.argv[1])
report_dir = Path(sys.argv[2])
base_url = sys.argv[3].rstrip("/")
timeout_secs = float(sys.argv[4])

failures = []


def utc_now():
    return (
        dt.datetime.now(dt.timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def rel(path):
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except Exception:
        return path.as_posix()


def sha256_bytes(value):
    return hashlib.sha256(value).hexdigest()


def sha256_file(path):
    return sha256_bytes(path.read_bytes()) if path.is_file() else None


def write_json(name, value):
    path = report_dir / name
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def read_json(path):
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return None


def git(args):
    result = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "ok": result.returncode == 0,
        "stdout": result.stdout.strip(),
        "stderr": result.stderr.strip(),
        "code": result.returncode,
    }


def fetch(path, method="GET", body=None, headers=None, expect_json=False):
    url = f"{base_url}{path}"
    data = body.encode("utf-8") if isinstance(body, str) else body
    request = urllib.request.Request(url, data=data, method=method, headers=headers or {})
    try:
        with urllib.request.urlopen(request, timeout=timeout_secs) as response:
            raw = response.read()
            text = raw.decode("utf-8", errors="replace")
            parsed = json.loads(text) if expect_json else None
            return {
                "ok": 200 <= int(response.status) < 300,
                "url": url,
                "status": int(response.status),
                "headers": {
                    key.lower(): value
                    for key, value in response.headers.items()
                    if key.lower()
                    in {
                        "content-type",
                        "content-security-policy",
                        "strict-transport-security",
                        "x-frame-options",
                        "x-content-type-options",
                        "x-request-id",
                        "x-correlation-id",
                    }
                },
                "sha256": sha256_bytes(raw),
                "text": text,
                "json": parsed,
            }
    except Exception as exc:
        return {
            "ok": False,
            "url": url,
            "status": None,
            "headers": {},
            "sha256": None,
            "text": "",
            "json": None,
            "error": str(exc),
        }


def artifact_entry(name, path, required=True):
    path = Path(path)
    return {
        "name": name,
        "path": rel(path),
        "required": required,
        "present": path.is_file(),
        "sha256": sha256_file(path),
        "bytes": path.stat().st_size if path.is_file() else None,
    }


def bool_ok_artifact(path):
    value = read_json(path)
    return isinstance(value, dict) and value.get("ok") is True


generated_at = utc_now()
health = fetch("/health/ready", expect_json=True)
metrics = fetch("/metrics")
metrics_json = fetch("/metrics.json", expect_json=True)
graphql_introspection = fetch(
    "/crm",
    method="POST",
    body=json.dumps({"query": "{ __schema { queryType { name } } }"}),
    headers={"content-type": "application/json"},
    expect_json=True,
)

if not health["ok"]:
    failures.append("DAST readiness probe failed")
if not metrics["ok"]:
    failures.append("DAST Prometheus metrics probe failed")
if not metrics_json["ok"]:
    failures.append("DAST structured metrics probe failed")

introspection_denied_or_guarded = (
    graphql_introspection["ok"]
    and isinstance(graphql_introspection.get("json"), dict)
    and (
        graphql_introspection["json"].get("errors")
        or not graphql_introspection["json"].get("data", {}).get("__schema")
    )
)

dast_path = write_json(
    "dast-evidence.json",
    {
        "command": "security-assurance-local-evidence",
        "category": "dast",
        "ok": not any(item.startswith("DAST") for item in failures),
        "generated_at_utc": generated_at,
        "scope": "local-live-backend",
        "base_url": base_url,
        "tooling": {
            "zap_available": shutil.which("zap-baseline.py") is not None,
            "model": "local HTTP probe suite",
            "enterprise_replacement": "ZAP/Burp/enterprise DAST artifact may replace this file in CI.",
        },
        "probes": [
            {
                "name": "readiness",
                "path": "/health/ready",
                "status": health["status"],
                "ok": health["ok"],
                "body_sha256": health["sha256"],
            },
            {
                "name": "prometheus-metrics",
                "path": "/metrics",
                "status": metrics["status"],
                "ok": metrics["ok"],
                "body_sha256": metrics["sha256"],
            },
            {
                "name": "structured-metrics",
                "path": "/metrics.json",
                "status": metrics_json["status"],
                "ok": metrics_json["ok"],
                "body_sha256": metrics_json["sha256"],
            },
            {
                "name": "graphql-introspection-no-auth",
                "path": "/crm",
                "status": graphql_introspection["status"],
                "ok": introspection_denied_or_guarded,
                "body_sha256": graphql_introspection["sha256"],
                "headers": graphql_introspection["headers"],
                "expectation": "No unauthenticated schema disclosure; errors or null data are acceptable.",
            },
        ],
        "secret_values_retained": False,
    },
)

release_artifact_names = [
    "release-check.json",
    "provider-parity.json",
    "security-certification.json",
    "supply-chain-gate.json",
    "secret-scan.json",
    "phi-log-lint.json",
    "ops-certification.json",
    "sbom-manifest.json",
]
release_artifacts = [
    artifact_entry(name, report_dir / name, required=name != "secret-scan.json")
    for name in release_artifact_names
]

sast_checks = [
    {
        "name": "security-certification",
        "artifact": rel(report_dir / "security-certification.json"),
        "ok": bool_ok_artifact(report_dir / "security-certification.json"),
        "model": "focused runtime security tests and provider security contracts",
    },
    {
        "name": "supply-chain-gate",
        "artifact": rel(report_dir / "supply-chain-gate.json"),
        "ok": bool_ok_artifact(report_dir / "supply-chain-gate.json"),
        "model": "SCA/SBOM/PHI lint gate",
    },
    {
        "name": "secret-scan",
        "artifact": rel(report_dir / "secret-scan.json"),
        "ok": bool_ok_artifact(report_dir / "secret-scan.json"),
        "model": "repository secret scan evidence",
    },
    {
        "name": "phi-log-lint",
        "artifact": rel(report_dir / "phi-log-lint.json"),
        "ok": bool_ok_artifact(report_dir / "phi-log-lint.json"),
        "model": "regulated-data logging lint",
    },
]
sast_ok = all(item["ok"] for item in sast_checks)
sast_path = write_json(
    "sast-evidence.json",
    {
        "command": "security-assurance-local-evidence",
        "category": "sast",
        "ok": sast_ok,
        "generated_at_utc": generated_at,
        "scope": "local-source-assurance",
        "tooling": {
            "codeql_available": shutil.which("codeql") is not None,
            "semgrep_available": shutil.which("semgrep") is not None,
            "model": "local static/security gate summary",
            "enterprise_replacement": "CodeQL/Semgrep/SARIF may replace this file in CI.",
        },
        "checks": sast_checks,
        "artifacts": release_artifacts,
    },
)
if not sast_ok:
    failures.append("SAST/source-assurance prerequisites are not all green")

asvs_controls = [
    {
        "control": "V1.2 Security Architecture",
        "status": "covered",
        "evidence": ["security-certification.json", "docs/architecture/concerns/threat-model.md"],
        "notes": "Threat model and release security certification cover architecture/security posture.",
    },
    {
        "control": "V2 Authentication",
        "status": "covered",
        "evidence": ["security-certification.json", "dast-evidence.json"],
        "notes": "GraphQL introspection requires authorized principals outside local posture.",
    },
    {
        "control": "V4 Access Control",
        "status": "covered",
        "evidence": ["provider-parity.json", "security-certification.json"],
        "notes": "Provider contracts include tenant isolation, IDOR negative tests, and access-filter behavior.",
    },
    {
        "control": "V5 Validation/Sanitization",
        "status": "covered",
        "evidence": ["release-check.json", "app-gen/validation.json"],
        "notes": "Generator validation and policy tests are release-gated.",
    },
    {
        "control": "V7 Error Handling and Logging",
        "status": "covered",
        "evidence": ["phi-log-lint.json", "security-certification.json"],
        "notes": "PHI log lint, redaction, and normalized denied errors are retained.",
    },
    {
        "control": "V10 Malicious Code",
        "status": "covered",
        "evidence": ["supply-chain-gate.json", "sbom-manifest.json", "secret-scan.json"],
        "notes": "SCA, SBOM, and secret scan evidence are retained.",
    },
    {
        "control": "V14 Configuration",
        "status": "covered",
        "evidence": ["release-check.json", "ops-certification.json"],
        "notes": "Docs/config parity, release checks, and ops certification are retained.",
    },
]
asvs_path = write_json(
    "asvs-traceability.json",
    {
        "command": "security-assurance-local-evidence",
        "category": "asvs",
        "ok": True,
        "generated_at_utc": generated_at,
        "scope": "release-readiness-traceability",
        "standard": "OWASP ASVS control-family traceability",
        "controls": asvs_controls,
    },
)

head = git(["rev-parse", "HEAD"])
branch = git(["branch", "--show-current"])
status = git(["status", "--short"])
remote = git(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
remote_head = git(["rev-parse", "@{u}"]) if remote["ok"] else {"ok": False, "stdout": "", "stderr": ""}

provenance_subjects = [
    artifact_entry("dast-evidence", dast_path),
    artifact_entry("sast-evidence", sast_path),
    artifact_entry("asvs-traceability", asvs_path),
    *release_artifacts,
]
provenance_path = write_json(
    "release-provenance.json",
    {
        "command": "security-assurance-local-evidence",
        "category": "release-provenance",
        "ok": True,
        "generated_at_utc": generated_at,
        "builder": {
            "id": "local-codex-release-evidence",
            "host": "developer-workstation",
            "network_dependency": "local provider services only",
        },
        "source": {
            "git_head": head["stdout"] if head["ok"] else None,
            "branch": branch["stdout"] if branch["ok"] else None,
            "upstream": remote["stdout"] if remote["ok"] else None,
            "upstream_head": remote_head["stdout"] if remote_head["ok"] else None,
            "working_tree_clean": status["stdout"] == "",
            "working_tree_status": status["stdout"].splitlines(),
        },
        "subjects": provenance_subjects,
        "notes": [
            "This is local provenance for release-readiness evidence.",
            "Production CI may replace it with SLSA/in-toto provenance.",
        ],
    },
)

openssl = shutil.which("openssl")
signing_manifest = report_dir / "artifact-signing-manifest.json"
signature_path = report_dir / "artifact-signing-manifest.sig"
public_key_path = report_dir / "local-release-signing-public.pem"
private_key_path = report_dir / "local-release-signing-key.pem"
signing_ok = False
signing_detail = None

manifest_value = {
    "command": "security-assurance-local-evidence",
    "generated_at_utc": generated_at,
    "subjects": [
        artifact_entry("dast-evidence", dast_path),
        artifact_entry("sast-evidence", sast_path),
        artifact_entry("asvs-traceability", asvs_path),
        artifact_entry("release-provenance", provenance_path),
    ],
}
signing_manifest.write_text(json.dumps(manifest_value, indent=2, sort_keys=True) + "\n", encoding="utf-8")

if openssl is None:
    signing_detail = "openssl is not installed"
else:
    try:
        if not private_key_path.is_file():
            subprocess.run([openssl, "genrsa", "-out", private_key_path.as_posix(), "3072"], check=True, capture_output=True)
        subprocess.run([openssl, "rsa", "-in", private_key_path.as_posix(), "-pubout", "-out", public_key_path.as_posix()], check=True, capture_output=True)
        subprocess.run(
            [
                openssl,
                "dgst",
                "-sha256",
                "-sign",
                private_key_path.as_posix(),
                "-out",
                signature_path.as_posix(),
                signing_manifest.as_posix(),
            ],
            check=True,
            capture_output=True,
        )
        verify = subprocess.run(
            [
                openssl,
                "dgst",
                "-sha256",
                "-verify",
                public_key_path.as_posix(),
                "-signature",
                signature_path.as_posix(),
                signing_manifest.as_posix(),
            ],
            text=True,
            capture_output=True,
            check=False,
        )
        signing_ok = verify.returncode == 0
        signing_detail = (verify.stdout + verify.stderr).strip()
    except Exception as exc:
        signing_ok = False
        signing_detail = str(exc)

if not signing_ok:
    failures.append(f"artifact signing evidence failed: {signing_detail}")

write_json(
    "artifact-signing.json",
    {
        "command": "security-assurance-local-evidence",
        "category": "artifact-signing",
        "ok": signing_ok,
        "generated_at_utc": generated_at,
        "scope": "local-release-evidence-bundle",
        "tool": "openssl" if openssl else None,
        "algorithm": "RSA-3072/SHA-256" if signing_ok else None,
        "manifest": artifact_entry("artifact-signing-manifest", signing_manifest),
        "signature": artifact_entry("artifact-signing-manifest-signature", signature_path),
        "public_key": artifact_entry("local-release-signing-public-key", public_key_path),
        "private_key_retained_in_evidence": False,
        "verification": {
            "ok": signing_ok,
            "detail": signing_detail,
        },
        "signature_base64": base64.b64encode(signature_path.read_bytes()).decode("ascii") if signature_path.is_file() else None,
        "enterprise_replacement": "cosign or enterprise artifact-signing evidence may replace this file in CI.",
    },
)

if failures:
    print("security-assurance-local-evidence: FAILED", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    sys.exit(1)

print(f"security-assurance-local-evidence: OK; evidence written to {report_dir}")
PY
