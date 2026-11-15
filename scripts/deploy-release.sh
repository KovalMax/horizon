#!/bin/sh
set -eu
cd "$(dirname "$0")"

compose() {
  docker compose \
    --env-file .env.production \
    --env-file .image-tag \
    --project-name horizon \
    --file compose.prod.yaml \
    "$@"
}

restore_previous_tag() {
  if [ -f .image-tag.previous ]; then
    mv .image-tag.previous .image-tag
  else
    rm -f .image-tag
  fi
}

check_readiness() {
  curl --fail --silent --show-error --connect-timeout 5 --max-time 15 \
    --retry 12 --retry-connrefused --retry-delay 5 \
    https://horizon.false.tel/api/health/ready
}

test -s .image-tag.next
if [ -f .image-tag ]; then
  cp .image-tag .image-tag.previous
else
  rm -f .image-tag.previous
fi
mv .image-tag.next .image-tag
. ./.image-tag

if ! compose config --quiet; then
  restore_previous_tag
  exit 1
fi

if ! compose pull api web; then
  restore_previous_tag
  exit 1
fi

if ! compose run --rm api migrate; then
  echo "Migration failed; application containers were not updated." >&2
  restore_previous_tag
  exit 1
fi

if ! compose up -d --no-build --remove-orphans; then
  echo "Compose rollout failed; restoring the previous image tag." >&2
  if [ -f .image-tag.previous ]; then
    restore_previous_tag
    compose pull api web
    compose up -d --no-build --remove-orphans
    if ! check_readiness; then
      echo "The previous release was restored but is not ready." >&2
    fi
  else
    restore_previous_tag
    echo "No previous release exists to roll back to." >&2
  fi
  exit 1
fi

if ! check_readiness; then
  echo "Readiness check failed; attempting an image rollback." >&2
  if [ -f .image-tag.previous ]; then
    restore_previous_tag
    compose pull api web
    compose up -d --no-build --remove-orphans
    if ! check_readiness; then
      echo "The previous release was restored but is not ready." >&2
    fi
    exit 1
  fi
  echo "No previous release tag exists; this appears to be the initial deployment." >&2
  exit 1
fi

echo "Release ${IMAGE_TAG} is deployed and ready."
