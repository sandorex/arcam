#!/usr/bin/env bash
# simple script to run all the test scripts quickly

FAILED=0
for file in ./*test_*.sh; do
    if [[ -x "$file" ]]; then
        if ! "$file"; then
            FAILED=1
        fi
        echo
    fi
done

exit "$FAILED"
