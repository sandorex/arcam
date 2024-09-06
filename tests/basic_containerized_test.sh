#!/usr/bin/env bash
# basic test experiment

set -e -o pipefail

if [[ ! -e /"${EXE:?}" ]]; then
    echo "EXE must be an absolute path!"
    exit 1
fi

function run_in_podman() {
    podman run --rm -i \
               --security-opt label=disable \
               --user podman \
               --privileged \
               --device /dev/fuse \
               --volume "${EXE:?}:/usr/bin/box:ro,nocopy" \
               --env "ARCAM_ENGINE=/usr/bin/podman" \
               --env "ARCAM_CONTAINER=test-box" \
               --env "USER=podman" \
               --env "HOSTNAME=podman" \
               quay.io/podman/stable "$@"
}

# print the version
"${EXE:?}" --version

cat <<'EOF' | run_in_podman bash -
set -ex

arcam start debian:bookworm-slim -- --net=private --uts=private

sleep 2s

arcam list

arcam exists

arcam exec -- stat .
EOF
