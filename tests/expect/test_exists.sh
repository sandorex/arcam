#!/usr/bin/env bash
# test exists command

set -e -o pipefail

source common.sh

export ARCAM_IMAGE="docker.io/library/debian:trixie-slim"

"$EXE" --version

run - <<'EOF'
    start_container $env(EXE) $env(ARCAM_IMAGE)
EOF

run_test "exists without name" - <<'EOF'
    spawn $env(EXE) exists
    expect eof
    wait_check_result "Did not detect the container"
EOF

run - <<'EOF'
    kill_container $env(EXE)
EOF

run_test "exists without running container" - <<'EOF'
    spawn $env(EXE) exists
    expect "running container"
    wait_check_result "Did not detect the container" 1
EOF

# start the container
run_test "exists with name" - <<'EOF'
    set container [start_container $env(EXE) $env(ARCAM_IMAGE)]

    spawn $env(EXE) exists $container
    wait_check_result "Did not detect the container"
EOF

run - <<'EOF'
    kill_container $env(EXE)
EOF

