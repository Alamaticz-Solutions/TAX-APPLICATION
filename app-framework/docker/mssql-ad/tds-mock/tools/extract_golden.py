#!/usr/bin/env python3
"""Extract sanitized golden handshake metadata from a TDSDUMP log."""

import argparse
import json
import re
from pathlib import Path


def extract(path: Path) -> dict:
  text = path.read_text(encoding="utf-8", errors="replace")
  flags_match = re.search(r"negotiate flags.*?(0x[0-9a-fA-F]+)", text)
  has_negotiate = "NTLMSSP_NEGOTIATE" in text or "using NTLM authentication" in text
  has_challenge = "NTLMSSP_CHALLENGE" in text or "NTLMSSP" in text
  return {
    "description": "Captured from TDSDUMP; credentials stripped",
    "challenge_flags": int(flags_match.group(1), 16) if flags_match else None,
    "target_info_hex": None,
    "token_sequence": [
      "prelogin",
      "login7",
      "sspi_challenge" if has_challenge else "sspi_challenge_unknown",
      "sspi_auth" if has_negotiate else "sspi_auth_unknown",
      "loginack",
      "sql_batch",
    ],
  }


def main() -> int:
  parser = argparse.ArgumentParser()
  parser.add_argument("tdsdump", type=Path)
  parser.add_argument("-o", "--output", type=Path, required=True)
  args = parser.parse_args()
  data = extract(args.tdsdump)
  args.output.parent.mkdir(parents=True, exist_ok=True)
  args.output.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
  print(f"written {args.output}")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
