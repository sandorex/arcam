#!/usr/bin/env bash
# test start command

set -eu -o pipefail

source common.sh

# using env vars to set everything as it requires less code
export ARCAM_IMAGE="docker.io/library/debian:trixie-slim"
export ARCAM_DIR="$(pwd)"

"$EXE" --version

run_test "start with random name" - <<'EOF'
    spawn $env(EXE) start
    expect {
        -re {^([A-Za-z\-]+)\r\n$} { }
        "already running" { exit 2 }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait

    set name $expect_out(1,string)
    spawn podman kill $name
    expect {
        -ex "$name\r\n" {}
        timeout { exit 2 }
        eof { exit 2 }
    }
    wait
EOF

export ARCAM_CONTAINER='arcam-test'

run_test "plain start" - <<'EOF'
    spawn $env(EXE) start
    expect {
        -ex "$env(ARCAM_CONTAINER)\r\n" {}
        "already running" { exit 2 }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait
EOF

podman stop "$ARCAM_CONTAINER" >/dev/null

run_test "start with config @test" - <<'EOF'
    spawn $env(EXE) start @test
    expect {
        -ex "$env(ARCAM_CONTAINER)\r\n" {}
        "already running" { exit 2 }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait
EOF

podman stop "$ARCAM_CONTAINER" >/dev/null
