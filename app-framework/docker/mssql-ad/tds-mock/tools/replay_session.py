#!/usr/bin/env python3
"""Replay captured TDS session bytes and print server responses."""
import os
import socket
import struct
import sys

HOST = os.environ.get("TDS_MOCK_HOST", "tds-mock.appfw.test")
PORT = int(os.environ.get("TDS_MOCK_PORT", "1433"))

PRELOGIN = bytes.fromhex(
    "1201003a0000000000001a00060100200001020021000c03002d00040400310001ff0900000000004d5353514c536572766572000800000000"
)

# LOGIN7 with embedded NTLM negotiate (from quiet send - extract from tsql binary is hard)
# Use SSPI follow-on only after server challenge - skip full replay

SQL_BATCH = bytes.fromhex(
    "0101003000000100160000001200000200000000000000010000530045004c00450043005400200031000a00"
)


def recv_packet(sock: socket.socket) -> bytes | None:
    header = sock.recv(8)
    if len(header) < 8:
        return None
    length = struct.unpack(">H", header[2:4])[0]
    body = b""
    while len(body) < length - 8:
        chunk = sock.recv(length - 8 - len(body))
        if not chunk:
            break
        body += chunk
    return header + body


def hexdump(data: bytes) -> str:
    return data.hex()


def main() -> int:
    # This script needs authenticated session; run via docker exec on test-runner with TDSDUMP-derived login
    print("connecting to", HOST, PORT)
    sock = socket.create_connection((HOST, PORT), timeout=10)
    sock.sendall(PRELOGIN)
    pkt = recv_packet(sock)
    print("prelogin rsp", len(pkt or b""), hexdump(pkt or b"")[:80])

    # Read login7 from file if provided
    login_path = os.environ.get("LOGIN7_PACKET_FILE")
    if not login_path:
        print("set LOGIN7_PACKET_FILE to continue full replay")
        sock.close()
        return 0

    login7 = open(login_path, "rb").read()
    sock.sendall(login7)
    pkt = recv_packet(sock)
    print("challenge", len(pkt or b""), hexdump(pkt or b"")[:80])

    auth_path = os.environ.get("AUTH_PACKET_FILE")
    if auth_path:
        auth = open(auth_path, "rb").read()
        sock.sendall(auth)
        pkt = recv_packet(sock)
        print("login rsp", len(pkt or b""), hexdump(pkt or b"")[:80])

        sock.sendall(SQL_BATCH)
        pkt = recv_packet(sock)
        print("sql rsp", len(pkt or b""), hexdump(pkt or b""))
    sock.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
