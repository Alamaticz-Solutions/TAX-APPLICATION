#!/usr/bin/env bash
set -euo pipefail

detect_node_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}:${arch}" in
    Linux:x86_64)
      printf 'linux-x64\n'
      ;;
    Linux:aarch64|Linux:arm64)
      printf 'linux-arm64\n'
      ;;
    Darwin:x86_64)
      printf 'darwin-x64\n'
      ;;
    Darwin:arm64)
      printf 'darwin-arm64\n'
      ;;
    *)
      echo "unsupported Node platform ${os}:${arch}; set APPFW_NODE_PLATFORM explicitly" >&2
      return 1
      ;;
  esac
}

node_version="${APPFW_NODE_VERSION:-22.17.0}"
node_platform="${APPFW_NODE_PLATFORM:-$(detect_node_platform)}"
install_dir="${APPFW_NODE_INSTALL_DIR:-/usr/local}"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/appfw-node.XXXXXX")"

cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

archive="node-v${node_version}-${node_platform}.tar.xz"
base_url="https://nodejs.org/dist/v${node_version}"

command -v curl >/dev/null 2>&1 || {
  echo "curl is required to install Node ${node_version}" >&2
  exit 1
}
command -v sha256sum >/dev/null 2>&1 || {
  echo "sha256sum is required to verify Node ${node_version}" >&2
  exit 1
}
command -v tar >/dev/null 2>&1 || {
  echo "tar is required to install Node ${node_version}" >&2
  exit 1
}

cd "$tmp_dir"
curl -fsSLO "${base_url}/${archive}"
curl -fsSLO "${base_url}/SHASUMS256.txt"
grep " ${archive}\$" SHASUMS256.txt | sha256sum -c -

mkdir -p "$install_dir"
tar -xJf "$archive" -C "$install_dir" --strip-components=1

export PATH="${install_dir}/bin:${PATH}"
node --version
npm --version
