#!/usr/bin/env bash
set -euo pipefail

bucket="${1:-tabulensis-downloads}"
artifact_dir="${2:-.}"

assets=(
  "tabulensis-latest-windows-x86_64.exe"
  "tabulensis-latest-windows-x86_64.zip"
  "tabulensis-latest-windows-x86_64.exe.sha256"
  "tabulensis-latest-windows-x86_64.zip.sha256"
  "tabulensis-latest-macos-universal.tar.gz"
  "tabulensis-latest-macos-universal.tar.gz.sha256"
  "tabulensis-latest-linux-x86_64.tar.gz"
  "tabulensis-latest-linux-x86_64.tar.gz.sha256"
)

uploaded=0
missing=0

for a in "${assets[@]}"; do
  path="${artifact_dir}/${a}"
  if [[ ! -f "${path}" ]]; then
    echo "Skipping missing artifact: ${path}" >&2
    missing=1
    continue
  fi

  ct="application/octet-stream"
  if [[ "${a}" == *.zip ]]; then
    ct="application/zip"
  elif [[ "${a}" == *.tar.gz ]]; then
    ct="application/gzip"
  elif [[ "${a}" == *.sha256 ]]; then
    ct="text/plain; charset=utf-8"
  fi

  echo "Uploading ${a} -> r2://${bucket}/${a}"
  npx wrangler r2 object put "${bucket}/${a}" \
    --file "${path}" \
    --remote \
    --content-type "${ct}" >/dev/null

  uploaded=1
done

if [[ "${uploaded}" == "0" ]]; then
  echo "No artifacts found to upload in: ${artifact_dir}" >&2
  exit 2
fi

if [[ "${missing}" != "0" ]]; then
  echo "Note: some artifacts were missing; only uploaded files that existed." >&2
fi

echo "Done."
