#!/usr/bin/env bash
# test exec command

set -e -o pipefail

source common.sh

export ARCAM_IMAGE="docker.io/library/debian:trixie-slim"

"$EXE" --version

# start the container
run - <<'EOF'
    start_container $env(EXE) $env(ARCAM_IMAGE)
EOF

run_test "plain exec" - <<'EOF'
    spawn $env(EXE) exec -- stat -c "%n %F" /
    expect {
        -ex "/ directory" { }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait
EOF

run_test "exec with shell" - <<'EOF'
    spawn $env(EXE) exec --shell -- stat -c \"%n %F" / \; echo \$SHELL
    expect {
        -ex "/ directory\r\n/bin/sh\r\n" { }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait
EOF

run_test "exec with explicit shell" - <<'EOF'
    spawn $env(EXE) exec --shell=/bin/bash -- stat -c \"%n %F" / \; echo \$SHELL
    expect {
        -ex "/ directory\r\n/bin/bash\r\n" { }
        timeout { exit 1 }
        eof { exit 1 }
    }
    wait
EOF

# kill the container
run - <<'EOF'
    kill_container $env(EXE)
EOF

