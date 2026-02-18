#!/usr/bin/env bash
set -euo pipefail

# Maintainer-only operational script for Cardthropic.
# Not intended as a stable public interface for third-party use.

usage() {
  cat <<'EOF'
Usage:
  scripts/ci/upload-artifacts.sh [artifact_path]

Behavior:
  - Uploads the artifact bundle using one configured backend.
  - If no backend is configured, prints a skip message and exits 0.

Backend 1 (generic HTTP PUT):
  ARTIFACT_UPLOAD_URL   required endpoint URL
  ARTIFACT_UPLOAD_TOKEN optional bearer token

Backend 2 (AWS S3 via aws cli):
  ARTIFACT_S3_URI       required destination URI (e.g. s3://bucket/path/file.tgz)
  AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY / AWS_DEFAULT_REGION
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

ARTIFACT_PATH="${1:-reports/ci-bundle.tgz}"

if [[ ! -f "${ARTIFACT_PATH}" ]]; then
  echo "Artifact missing: ${ARTIFACT_PATH}" >&2
  exit 2
fi

if [[ -n "${ARTIFACT_UPLOAD_URL:-}" ]]; then
  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required for ARTIFACT_UPLOAD_URL mode." >&2
    exit 2
  fi
  echo "Uploading artifact via HTTP PUT to configured endpoint."
  auth_args=()
  if [[ -n "${ARTIFACT_UPLOAD_TOKEN:-}" ]]; then
    auth_args=(-H "Authorization: Bearer ${ARTIFACT_UPLOAD_TOKEN}")
  fi
  curl --fail --show-error --silent \
    -X PUT \
    "${auth_args[@]}" \
    -H "Content-Type: application/gzip" \
    --data-binary @"${ARTIFACT_PATH}" \
    "${ARTIFACT_UPLOAD_URL}"
  echo "Artifact upload complete."
  exit 0
fi

if [[ -n "${ARTIFACT_S3_URI:-}" ]]; then
  if ! command -v aws >/dev/null 2>&1; then
    echo "aws CLI is required for ARTIFACT_S3_URI mode." >&2
    exit 2
  fi
  echo "Uploading artifact to S3 URI: ${ARTIFACT_S3_URI}"
  aws s3 cp "${ARTIFACT_PATH}" "${ARTIFACT_S3_URI}"
  echo "Artifact upload complete."
  exit 0
fi

echo "No artifact upload backend configured; skipping."
