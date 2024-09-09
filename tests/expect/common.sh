#!/usr/bin/env bash
# contains common bash functions

if [[ -v container ]]; then
    echo "Cannot run tests in nested podman currently"
    exit 1
fi

if ! command -v podman &>/dev/null; then
    echo "Podman not found in PATH"
    exit 1
fi

if ! command -v expect &>/dev/null; then
    echo "Expect not found in PATH"
    exit 1
fi

# NOTE args here are those passed to the script itself
export TEST="$(basename "$0")"
export EXE="${1:-../../target/debug/arcam}"
FAILED=0

# run expect with common functions sourced
# shellcheck disable=SC2120
function run() {
    # disable it as it makes the script fail prematurely
    set +e

    echo

    expect -c "source common.tcl" "$@"
    ret=$?

    # run is not meant for tests so errors here are a failure in the system
    if [[ "$ret" -ne 0 ]]; then
        # quit, there was unknown error in setup
        FAILED=2
        exit
    fi
}

# run expect as a test, if it fails it wont stop script execution
function run_test() {
    # disable it as it makes the script fail prematurely
    set +e

    echo

    echo -e "$(tput bold)$(tput setaf 3)Test: ${1:?}$(tput sgr0)"
    shift

    expect -c "source common.tcl" "$@"
    ret=$?

    if [[ "$ret" -eq 0 ]]; then
        echo -e "$(tput bold)$(tput setaf 2)Test Succeded$(tput sgr0)"
    elif [[ "$ret" -eq 1 ]]; then
        echo -e "$(tput bold)$(tput setaf 1)Test Failed$(tput sgr0) ${err_msg:-}"
        FAILED=1
    else
        # quit, there was unknown error in setup
        FAILED=2
        exit
    fi
}

# print result and exit with proper code
function end_test_suite() {
    if [[ "$FAILED" -eq 0 ]]; then
        # all tests succeded
        echo -e "$(tput bold)$(tput setaf 2)Test Script $TEST Succeded$(tput sgr0)"
    elif [[ "$FAILED" -eq 1 ]]; then
        # one of the tests failed
        echo -e "$(tput bold)$(tput setaf 1)Test Script $TEST Failed$(tput sgr0)"
    else
        # something is wrong, just abort tests
        echo -e "$(tput bold)$(tput setaf 1)Test Script $TEST Panicked$(tput sgr0)"
    fi
    exit "$FAILED"
}

# call it automatically to reduce boilerplate
trap end_test_suite EXIT

