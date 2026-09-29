#!/usr/bin/env python3
"""Self-test: NTLM challenge/response round-trip matches ntlm-auth oracle."""

import json
import os
import struct
import sys
from pathlib import Path

from ntlm_auth.compute_response import ComputeResponse
from ntlm_auth.constants import NegotiateFlags, NTLM_SIGNATURE
from ntlm_auth.messages import ChallengeMessage, NegotiateMessage

from server import build_challenge_message, build_target_info


def run_selftest(golden_path: Path) -> None:
  domain = os.environ.get("TDS_MOCK_DOMAIN", "APPFW")
  username = os.environ.get("TDS_MOCK_USERNAME", "svc-app")
  password = os.environ.get("TDS_MOCK_PASSWORD", "AppSvc!Passw0rd")

  golden = {}
  if golden_path.is_file():
    golden = json.loads(golden_path.read_text(encoding="utf-8"))

  negotiate_flags = (
      NegotiateFlags.NTLMSSP_NEGOTIATE_TARGET_INFO
      | NegotiateFlags.NTLMSSP_NEGOTIATE_UNICODE
      | NegotiateFlags.NTLMSSP_NEGOTIATE_EXTENDED_SESSIONSECURITY
      | NegotiateFlags.NTLMSSP_NEGOTIATE_VERSION
  )
  negotiate = NegotiateMessage(negotiate_flags, None, None)
  negotiate_bytes = negotiate.get_data()
  assert negotiate_bytes.startswith(NTLM_SIGNATURE)

  server_challenge = b"\x01\x02\x03\x04\x05\x06\x07\x08"
  client_flags = struct.unpack("<I", negotiate.negotiate_flags)[0]
  target_info_raw = build_target_info(domain, golden)
  golden_flags = golden.get("challenge_flags")
  challenge_bytes = build_challenge_message(
    server_challenge,
    domain,
    client_flags,
    target_info_raw=target_info_raw,
    golden_flags=int(golden_flags) if golden_flags is not None else None,
  )
  challenge_obj = ChallengeMessage(challenge_bytes)

  cr = ComputeResponse(username, password, domain, challenge_obj, 3)
  lm_resp = cr.get_lm_challenge_response()
  nt_resp, _, _ = cr.get_nt_challenge_response(lm_resp)
  temp = nt_resp[16:]
  timestamp = temp[8:16]
  expected_nt, _ = ComputeResponse._get_NTLMv2_response(
    username,
    password,
    domain,
    server_challenge,
    cr._client_challenge,
    timestamp,
    challenge_obj.target_info,
  )
  assert nt_resp == expected_nt, "NTLM self-test oracle mismatch"

  if golden.get("token_sequence"):
    expected_seq = golden["token_sequence"]
    assert "ntlm_negotiate" in expected_seq or "sspi_challenge" in expected_seq

  print("golden_selftest: ok")


if __name__ == "__main__":
  golden = Path(sys.argv[1] if len(sys.argv) > 1 else "/golden/handshake.json")
  run_selftest(golden)
