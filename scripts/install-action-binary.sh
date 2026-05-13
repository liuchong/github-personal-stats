#!/usr/bin/env bash
set -euo pipefail

repo="${GITHUB_ACTION_REPOSITORY:-liuchong/github-personal-stats}"
version="${INPUT_VERSION:-}"
binary_url="${INPUT_BINARY_URL:-}"
os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "${os}" in
  darwin) os="macos" ;;
esac

case "${arch}" in
  x86_64) arch="x64" ;;
  arm64|aarch64) arch="arm64" ;;
  *) echo "unsupported architecture: ${arch}" >&2; exit 1 ;;
esac

if [ -z "${version}" ]; then
  version="latest"
fi

if [ -z "${binary_url}" ]; then
  if [ "${version}" = "latest" ]; then
    binary_url="https://github.com/${repo}/releases/latest/download/github-personal-stats-${os}-${arch}.tar.gz"
    checksum_url="https://github.com/${repo}/releases/latest/download/checksums.txt"
  else
    binary_url="https://github.com/${repo}/releases/download/${version}/github-personal-stats-${os}-${arch}.tar.gz"
    checksum_url="https://github.com/${repo}/releases/download/${version}/checksums.txt"
  fi
else
  checksum_url=""
fi

temp_dir="${RUNNER_TEMP:-/tmp}/github-personal-stats-action"
bin_dir="${temp_dir}/bin"
archive="${temp_dir}/github-personal-stats.tar.gz"
mkdir -p "${bin_dir}"

curl --fail --location --silent --show-error "${binary_url}" --output "${archive}"

if [ -n "${checksum_url}" ]; then
  curl --fail --location --silent --show-error "${checksum_url}" --output "${temp_dir}/checksums.txt"
  expected="$(grep "github-personal-stats-${os}-${arch}.tar.gz" "${temp_dir}/checksums.txt" | awk '{print $1}')"
  actual="$(shasum -a 256 "${archive}" | awk '{print $1}')"
  if [ "${expected}" != "${actual}" ]; then
    echo "checksum mismatch" >&2
    exit 1
  fi
fi

tar -xzf "${archive}" -C "${bin_dir}"
chmod +x "${bin_dir}/github-personal-stats"
echo "${bin_dir}" >> "${GITHUB_PATH}"
