#!/usr/bin/env python3
"""Minimal TDS 7.4 server with real NTLMv2 verification for hermetic NTLM certification."""

from __future__ import annotations

import argparse
import json
import logging
import os
import socket
import struct
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

from ntlm_auth.compute_response import ComputeResponse
from ntlm_auth.constants import AvId, MessageTypes, NegotiateFlags, NTLM_SIGNATURE
from ntlm_auth.messages import ChallengeMessage, TargetInfo, get_version

LOG = logging.getLogger("tds-mock")

TDS_PRELOGIN = 0x12
TDS_LOGIN7 = 0x10
TDS_SSPI = 0x11
TDS_SQL_BATCH = 0x01
TDS_RPC = 0x03
TDS_TABULAR = 0x04

TOKEN_ERROR = 0xAA
TOKEN_LOGINACK = 0xAD
TOKEN_SSPI = 0xED
TOKEN_DONE = 0xFD
TOKEN_COLMETADATA = 0x81
TOKEN_ROW = 0xD1
TOKEN_ENVCHANGE = 0xE3

def tds_packet(msg_type: int, payload: bytes, status: int = 0x01) -> bytes:
    length = len(payload) + 8
    header = struct.pack(">BBHHBB", msg_type, status, length, 0, 1, 0)
    return header + payload


def prelogin_response(client_data: bytes) -> bytes:
  """Mirror client PRELOGIN options with valid VERSION/ENCRYPTION responses."""
  opt_map: list[tuple[int, int, int]] = []
  idx = 0
  while idx < len(client_data):
    token = client_data[idx]
    idx += 1
    if token == 0xFF:
      break
    if idx + 4 > len(client_data):
      break
    offset = struct.unpack_from(">H", client_data, idx)[0]
    length = struct.unpack_from(">H", client_data, idx + 2)[0]
    idx += 4
    opt_map.append((token, offset, length))

  chunks: list[bytes] = []
  for token, client_offset, client_length in opt_map:
    if token == 0x00:  # VERSION
      chunks.append(bytes([0x0a, 0x00, 0x00, 0x00, 0x00, 0x00]))
    elif token == 0x01:  # ENCRYPTION
      chunks.append(bytes([0x02]))  # ENCRYPT_NOT_SUP — skip TLS, plaintext LOGIN7
    elif token == 0x02:  # INSTOPT
      if client_length and client_offset + client_length <= len(client_data):
        chunks.append(client_data[client_offset:client_offset + client_length])
      else:
        chunks.append(b"")
    elif token == 0x03:  # THREAD
      if client_length and client_offset + client_length <= len(client_data):
        chunks.append(client_data[client_offset:client_offset + client_length])
      else:
        chunks.append(b"\x00\x00\x00\x00")
    elif token == 0x04:  # MARS
      if client_length and client_offset + client_length <= len(client_data):
        chunks.append(client_data[client_offset:client_offset + client_length])
      else:
        chunks.append(bytes([0x00]))
    else:
      chunks.append(b"\x00" * client_length)

  header_size = len(opt_map) * 5 + 1
  headers = bytearray()
  data_offset = header_size
  for i, (token, _, _) in enumerate(opt_map):
    chunk = chunks[i]
    headers.append(token)
    headers.extend(struct.pack(">H", data_offset))
    headers.extend(struct.pack(">H", len(chunk)))
    data_offset += len(chunk)
  headers.append(0xFF)

  body = bytearray()
  for chunk in chunks:
    body.extend(chunk)
  return tds_packet(TDS_TABULAR, bytes(headers) + bytes(body))


def error_token(number: int, state: int, severity: int, message: str) -> bytes:
  msg_bytes = message.encode("utf-16-le")
  server = "mssql".encode("utf-16-le")
  body = struct.pack("<I", number)
  body += struct.pack("<BB", state, severity)
  body += struct.pack("<H", len(message))
  body += msg_bytes
  body += struct.pack("<B", len("mssql"))
  body += server
  body += struct.pack("<B", 0)  # ProcNameLen
  body += struct.pack("<H", 1)  # LineNumber
  return struct.pack("<BH", TOKEN_ERROR, len(body)) + body


def login_error_done_token() -> bytes:
  # DONE token from SQL Server 2022 failed-login capture.
  return bytes.fromhex("fd020000000000000000000000")


def done_token(status: int = 0) -> bytes:
  # DONE token: status, curcmd, donecount (8 bytes) + optional info
  body = struct.pack("<III", status, 0, 0)
  return struct.pack("<BH", TOKEN_DONE, len(body)) + body


def login_success_tokens() -> bytes:
  # Byte-exact login token stream from SQL Server 2022 TDSDUMP (sa sql_password login).
  return bytes.fromhex(
    "e31b0001066d0061007300740065007200066d0061007300740065007200"
    "ab620045160000020025004300680061006e00670065006400200064006100740061006200610073006500200063006f006e007400650078007400200074006f00200027006d006100730074006500720027002e00056d007300730071006c000001000000"
    "e3080007050904d0003400"
    "e31700020a750073005f0065006e0067006c0069007300680000"
    "ab660047160000010027004300680061006e0067006500640020006c0061006e00670075006100670065002000730065007400740069006e006700200074006f002000750073005f0065006e0067006c006900730068002e00056d007300730071006c000001000000"
    "ad36000174000004164d006900630072006f0073006f00660074002000530051004c00200053006500720076006500720000000000100010a9"
    "e3130004043400300039003600043400300039003600"
    "ae0a0100000001ff"
    "fd000000000000000000000000"
  )


def envchange_database(db: str) -> bytes:
  new = db.encode("utf-16-le")
  old = b""
  body = struct.pack("<B", 1) + bytes([len(old)]) + old + bytes([len(new)]) + new
  return struct.pack("<BH", TOKEN_ENVCHANGE, len(body)) + body


def sspi_token(blob: bytes) -> bytes:
  return struct.pack("<BH", TOKEN_SSPI, len(blob)) + blob


def select_one_tokens() -> bytes:
  # Byte-exact from FreeTDS TDSDUMP against SQL Server 2022 SELECT 1 (TLS path).
  return bytes.fromhex(
    "8101000000000020003800d101000000fd1000c10001000000000000000000"
  )


def find_ntlm_blob(data: bytes) -> Optional[bytes]:
  idx = data.find(NTLM_SIGNATURE)
  if idx == -1:
    return None
  return data[idx:]


def build_target_info(domain: str, golden: Optional[dict[str, Any]] = None) -> bytes:
  """Build NTLM target info AVP list (MsvAvNbDomainName + terminator)."""
  if golden and golden.get("target_info_hex"):
    return bytes.fromhex(golden["target_info_hex"])
  domain_utf16 = domain.encode("utf-16-le")
  nb = b"\x02\x00" + struct.pack("<H", len(domain_utf16)) + domain_utf16
  eos = b"\x00\x00\x00\x00"
  return nb + eos


def build_challenge_message(
  server_challenge: bytes,
  domain: str,
  negotiate_flags: int,
  target_info_raw: Optional[bytes] = None,
  golden_flags: Optional[int] = None,
) -> bytes:
  target_name = domain.encode("utf-16-le")
  ti = TargetInfo()
  if target_info_raw:
    ti.unpack(target_info_raw)
  else:
    ti[AvId.MSV_AV_NB_DOMAIN_NAME] = domain.encode("utf-16-le")
  target_info_packed = ti.pack()

  flags = negotiate_flags | NegotiateFlags.NTLMSSP_REQUEST_TARGET | \
      NegotiateFlags.NTLMSSP_NEGOTIATE_TARGET_INFO | NegotiateFlags.NTLMSSP_NEGOTIATE_UNICODE
  if golden_flags is not None:
    flags = golden_flags

  body_len = 48
  if flags & NegotiateFlags.NTLMSSP_NEGOTIATE_VERSION:
    body_len = 56

  payload_offset = body_len
  target_name_offset = payload_offset
  payload_offset += len(target_name)
  target_info_offset = payload_offset
  payload_offset += len(target_info_packed)

  msg = bytearray()
  msg += NTLM_SIGNATURE
  msg += struct.pack("<I", MessageTypes.NTLM_CHALLENGE)
  msg += struct.pack("<HHI", len(target_name), len(target_name), target_name_offset)
  msg += struct.pack("<I", flags)
  msg += server_challenge
  msg += b"\x00" * 8
  msg += struct.pack("<HHI", len(target_info_packed), len(target_info_packed), target_info_offset)
  if flags & NegotiateFlags.NTLMSSP_NEGOTIATE_VERSION:
    msg += get_version(flags)
  assert len(msg) == body_len, f"challenge header length {len(msg)} != {body_len}"
  msg += target_name
  msg += target_info_packed
  return bytes(msg)


def extract_ntlm_field(blob: bytes, field_offset: int) -> bytes:
  length = struct.unpack_from("<H", blob, field_offset)[0]
  offset = struct.unpack_from("<I", blob, field_offset + 4)[0]
  return blob[offset:offset + length]


def parse_negotiate_flags(blob: bytes) -> int:
  return struct.unpack("<I", blob[12:16])[0]


class NtlmVerifier:
  def __init__(
    self,
    domain: str,
    username: str,
    password: str,
    golden: Optional[dict[str, Any]] = None,
  ) -> None:
    self.domain = domain.upper()
    self.username = username
    self.password = password
    self.golden = golden or {}
    self.server_challenge: Optional[bytes] = None
    self.challenge_message: Optional[ChallengeMessage] = None
    self.negotiate_flags: Optional[int] = None
    self.evidence: dict[str, Any] = {}

  def challenge_for_negotiate(self, blob: bytes) -> bytes:
    self.negotiate_flags = parse_negotiate_flags(blob)
    self.server_challenge = os.urandom(8)
    target_info_raw = build_target_info(self.domain, self.golden)
    golden_flags = self.golden.get("challenge_flags")
    challenge_bytes = build_challenge_message(
      self.server_challenge,
      self.domain,
      self.negotiate_flags,
      target_info_raw=target_info_raw,
      golden_flags=int(golden_flags) if golden_flags is not None else None,
    )
    self.challenge_message = ChallengeMessage(challenge_bytes)
    return challenge_bytes

  def verify_authenticate(self, blob: bytes) -> bool:
    if self.challenge_message is None or self.server_challenge is None:
      return False

    nt_blob = extract_ntlm_field(blob, 20)
    if len(nt_blob) < 24:
      return False

    temp = nt_blob[16:]
    timestamp = temp[8:16]
    client_challenge = temp[16:24]
    target_info = self.challenge_message.target_info or TargetInfo()

    user_bytes = extract_ntlm_field(blob, 36)
    domain_bytes = extract_ntlm_field(blob, 28)
    user = user_bytes.decode("utf-16-le") if user_bytes else self.username
    domain = domain_bytes.decode("utf-16-le") if domain_bytes else self.domain

    expected_nt, _ = ComputeResponse._get_NTLMv2_response(
      user,
      self.password,
      domain,
      self.server_challenge,
      client_challenge,
      timestamp,
      target_info,
    )
    verified = nt_blob == expected_nt
    self.evidence = {
      "verified_at": datetime.now(timezone.utc).isoformat(),
      "domain": domain,
      "username": user,
      "ntlmv2_verified": verified,
      "negotiate_flags": self.negotiate_flags,
      "target_info_keys": self._target_info_keys(target_info.pack()),
    }
    return verified

  def _target_info_keys(self, target_info: bytes) -> list[str]:
    keys: list[str] = []
    idx = 0
    while idx + 4 <= len(target_info):
      av_id, av_len = struct.unpack_from("<HH", target_info, idx)
      if av_id == 0:
        break
      keys.append(str(av_id))
      idx += 4 + av_len
    return keys


class TdsClientSession:
  def __init__(self, verifier: NtlmVerifier) -> None:
    self.verifier = verifier
    self.authenticated = False
    self.terminal = False
    self.database = "master"

  def handle(self, conn: socket.socket) -> None:
    buffer = b""
    while not self.terminal:
      chunk = conn.recv(4096)
      if not chunk:
        break
      buffer += chunk
      while len(buffer) >= 8:
        msg_type = buffer[0]
        status = buffer[1]
        length = struct.unpack_from(">H", buffer, 2)[0]
        if len(buffer) < length:
          break
        packet = buffer[:length]
        buffer = buffer[length:]
        self._dispatch(conn, msg_type, packet[8:])
        if self.terminal:
          return
        if not self.authenticated and msg_type in (TDS_LOGIN7, TDS_SSPI):
          continue

  def _dispatch(self, conn: socket.socket, msg_type: int, payload: bytes) -> None:
    LOG.debug("dispatch msg_type=0x%02x len=%d", msg_type, len(payload))
    if msg_type == TDS_PRELOGIN:
      conn.sendall(prelogin_response(payload))
      return

    if msg_type in (TDS_LOGIN7, TDS_SSPI):
      self._handle_login(conn, payload)
      return

    if msg_type == TDS_SQL_BATCH and self.authenticated:
      self._handle_sql(conn, payload)
      return

    if msg_type == TDS_RPC and self.authenticated:
      self._handle_rpc(conn, payload)

  def _handle_login(self, conn: socket.socket, payload: bytes) -> None:
    blob = find_ntlm_blob(payload)
    if blob is None:
      self._send_login_error(conn, 18456, "NTLM blob missing from login packet")
      return

    msg_type = struct.unpack("<I", blob[8:12])[0]
    if msg_type == 1:
      challenge = self.verifier.challenge_for_negotiate(blob)
      pkt = tds_packet(TDS_TABULAR, sspi_token(challenge))
      LOG.info("sending ntlm challenge (%d bytes)", len(pkt))
      conn.sendall(pkt)
      return

    if msg_type == 3:
      verified = self.verifier.verify_authenticate(blob)
      LOG.info("ntlm authenticate verified=%s", verified)
      if verified:
        self.authenticated = True
        tokens = login_success_tokens()
        pkt = tds_packet(TDS_TABULAR, tokens)
        LOG.info("sending login success (%d bytes)", len(pkt))
        conn.sendall(pkt)
      else:
        self._send_login_error(
          conn,
          18456,
          "Login failed. The login is from an untrusted domain and cannot be used with Integrated authentication.",
        )
      return

    self._send_login_error(conn, 18456, f"Unexpected NTLM message type {msg_type}")

  def _send_login_error(self, conn: socket.socket, number: int, message: str) -> None:
    LOG.info("sending login error %d", number)
    tokens = error_token(number, 1, 14, message) + login_error_done_token()
    conn.sendall(tds_packet(TDS_TABULAR, tokens))
    self.terminal = True

  def _handle_sql(self, conn: socket.socket, payload: bytes) -> None:
    LOG.info("sql batch received (%d bytes)", len(payload))
    self._send_select_one(conn)

  def _handle_rpc(self, conn: socket.socket, payload: bytes) -> None:
    LOG.info("rpc received (%d bytes)", len(payload))
    self._send_select_one(conn)

  def _send_select_one(self, conn: socket.socket) -> None:
    tokens = select_one_tokens()
    pkt = tds_packet(TDS_TABULAR, tokens)
    LOG.info("select 1 response (%d bytes)", len(pkt))
    conn.sendall(pkt)


def write_evidence(path: Path, evidence: dict[str, Any]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")


def load_golden(path: Optional[str]) -> Optional[dict[str, Any]]:
  if not path:
    return None
  golden_path = Path(path)
  if not golden_path.is_file():
    LOG.warning("golden file %s not found; using defaults", golden_path)
    return None
  return json.loads(golden_path.read_text(encoding="utf-8"))


def serve(host: str, port: int, verifier: NtlmVerifier, evidence_path: Path) -> None:
  sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
  sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
  sock.bind((host, port))
  sock.listen(20)
  LOG.info("tds-mock listening on %s:%d", host, port)

  def session_worker(conn: socket.socket, addr: tuple[str, int]) -> None:
    LOG.info("connection from %s", addr)
    session = TdsClientSession(NtlmVerifier(
      verifier.domain, verifier.username, verifier.password, verifier.golden))
    try:
      session.handle(conn)
      if session.authenticated and session.verifier.evidence:
        write_evidence(evidence_path, session.verifier.evidence)
    except Exception:
      LOG.exception("session error")
    finally:
      conn.close()

  while True:
    conn, addr = sock.accept()
    threading.Thread(target=session_worker, args=(conn, addr), daemon=True).start()


def main() -> int:
  parser = argparse.ArgumentParser(description="TDS/NTLM protocol-fidelity mock")
  parser.add_argument("--host", default=os.environ.get("TDS_MOCK_HOST", "0.0.0.0"))
  parser.add_argument("--port", type=int, default=int(os.environ.get("TDS_MOCK_PORT", "1433")))
  parser.add_argument("--domain", default=os.environ.get("TDS_MOCK_DOMAIN", "APPFW"))
  parser.add_argument("--username", default=os.environ.get("TDS_MOCK_USERNAME", "svc-app"))
  parser.add_argument("--password", default=os.environ.get("TDS_MOCK_PASSWORD", "AppSvc!Passw0rd"))
  parser.add_argument("--golden", default=os.environ.get("TDS_MOCK_GOLDEN", ""))
  parser.add_argument("--evidence", default=os.environ.get("TDS_MOCK_EVIDENCE", "/evidence/ntlm-handshake.json"))
  parser.add_argument("-v", action="store_true", help="verbose logging")
  args = parser.parse_args()

  logging.basicConfig(level=logging.DEBUG if args.v else logging.INFO)

  golden = load_golden(args.golden or None)
  verifier = NtlmVerifier(args.domain, args.username, args.password, golden)
  evidence_path = Path(args.evidence)

  serve(args.host, args.port, verifier, evidence_path)
  return 0


if __name__ == "__main__":
  sys.exit(main())
