#!/usr/bin/env python3
"""Probe tds-mock: login via tsql subprocess and read raw bytes after SELECT."""
import os
import socket
import struct
import subprocess
import sys
import time

HOST = os.environ.get("TDS_MOCK_HOST", "tds-mock.appfw.test")
PORT = int(os.environ.get("TDS_MOCK_PORT", "1433"))


def recv_exact(sock: socket.socket, n: int) -> bytes:
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            break
        buf += chunk
    return buf


def recv_packet(sock: socket.socket, timeout: float = 5.0) -> bytes | None:
    sock.settimeout(timeout)
    header = recv_exact(sock, 8)
    if len(header) < 8:
        return None
    length = struct.unpack(">H", header[2:4])[0]
    body = recv_exact(sock, length - 8)
    return header + body


def main() -> int:
    # Independent socket: send minimal prelogin then observe
    sock = socket.create_connection((HOST, PORT), timeout=5)
    # Minimal PRELOGIN request (VERSION + ENCRYPTION + terminator)
    prelogin = bytes.fromhex(
        "1201001e000000000000010000000000010000000000000000000000ff"
    )
    sock.sendall(prelogin)
    pkt = recv_packet(sock)
    print(f"prelogin response: {len(pkt or b'')} bytes")
    if pkt:
        print(pkt.hex())
    sock.close()

    # After tsql runs, check if server still accepts connections
    env = os.environ.copy()
    env["TDSVER"] = "7.4"
    cmd = [
        "tsql",
        "-H", HOST,
        "-p", str(PORT),
        "-U", "APPFW\\svc-app",
        "-P", "AppSvc!Passw0rd",
        "-D", "master",
    ]
    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
    )
    stdout, stderr = proc.communicate(input=b"SELECT 1\n", timeout=30)
    print("tsql exit:", proc.returncode)
    print("stdout:", stdout.decode(errors="replace"))
    print("stderr:", stderr.decode(errors="replace"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
