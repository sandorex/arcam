#!/usr/bin/env bash
# basic test to test box start

set -e -o pipefail

if [[ ! -e /"${BOX_EXE:?}" ]]; then
    echo "BOX_EXE must be an absolute path!"
    exit 1
fi

function run_in_podman() {
    podman run --rm -i \
               --security-opt label=disable \
               --user podman \
               --privileged \
               --device /dev/fuse \
               --volume "${BOX_EXE:?}:/usr/bin/box:ro,nocopy" \
               --env "BOX_ENGINE=/usr/bin/podman" \
               --env "BOX_CONTAINER=test-box" \
               --env "USER=podman" \
               --env "HOSTNAME=podman" \
               quay.io/podman/stable "$@"
}

# print the version
"${BOX_EXE:?}" --version

cat <<'EOF' | run_in_podman bash -
set -ex

box start debian:bookworm-slim -- --net=private --uts=private

sleep 2s

box list

box exists

box exec -- stat .
EOF
