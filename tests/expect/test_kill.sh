#!/usr/bin/env bash
# test kill command

set -eu -o pipefail

source common.sh

# using env vars to set everything as it requires less code
export ARCAM_IMAGE="docker.io/library/debian:trixie-slim"
export ARCAM_CONTAINER="arcam-test"

# if not killed succesfully then use fail and clean up
function cleanup() {
    if "$EXE" exists; then
        FAILED=1
        echo "Container was not killed"
        podman container stop "$ARCAM_CONTAINER" >/dev/null
        exit
    fi
}

"$EXE" --version

run - <<'EOF'
    start_container $env(EXE)
EOF

echo

run_test "kill without name" - <<'EOF'
    spawn $env(EXE) kill
    expect {
        -re {"([A-Za-z\-]+)".*\[[yY]/[nN]\]} { send "y\r"; send_user "\n" }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait

    set name $expect_out(1,string)
    if {$name != $env(ARCAM_CONTAINER)} {
        send_user "Wrong container found, $name != $env(ARCAM_CONTAINER)\n"
        exit 1
    }
EOF

cleanup

echo

run - <<'EOF'
    start_container $env(EXE)
EOF

echo

run_test "kill with name" - <<'EOF'
    spawn $env(EXE) kill $env(ARCAM_CONTAINER)
    expect {
        -re {"([A-Za-z\-]+)".*\[[yY]/[nN]\]} { send "y\r"; send_user "\n" }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait

    set name $expect_out(1,string)
    if {$name != $env(ARCAM_CONTAINER)} {
        send_user "Wrong container found, $name != $env(ARCAM_CONTAINER)\n"
        exit 1
    }
EOF

cleanup

echo

