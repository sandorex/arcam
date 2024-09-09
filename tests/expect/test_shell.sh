#!/usr/bin/env bash
# test shell command

set -e -o pipefail

source common.sh

export ARCAM_IMAGE="docker.io/library/debian:bookworm-slim"
export ARCAM_CONTAINER="arcam-test"

"$EXE" --version

run - <<'EOF'
    start_container $env(EXE) $env(ARCAM_IMAGE)
EOF

run_test "plain shell" - <<'EOF'
spawn arcam shell
expect {
    -re {$ *} { }
    timeout { exit 1 }
    eof { exit 1 }
}

send "stat -c '%n %F' /\r"
expect {
    -ex "/ directory" { }
    timeout { exit 1 }
    eof { exit 1 }
}

# note this also tests automatic shell detection
send "echo \$SHELL\r"
expect {
    -ex "/bin/bash" { }
    timeout { exit 1 }
    eof { exit 1 }
}

expect -re {$ *}
send "exit\r"
wait
EOF

run_test "shell with explicit shell" - <<'EOF'
spawn arcam shell $env(ARCAM_CONTAINER) /bin/sh
expect {
    -re {$ *} { }
    timeout { exit 1 }
    eof { exit 1 }
}

send "stat -c '%n %F' /\r"
expect {
    -ex "/ directory" { }
    timeout { exit 1 }
    eof { exit 1 }
}

send "echo \$SHELL\r"
expect {
    -ex "/bin/sh" { }
    timeout { exit 1 }
    eof { exit 1 }
}

expect -re {$ *}
send "exit\r"
wait
EOF

run - <<'EOF'
    kill_container $env(EXE)
EOF

