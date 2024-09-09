#!/usr/bin/env bash
# simple script to run all the test scripts quickly

FAILED=0
for file in ./*test_*.sh; do
    if [[ -x "$file" ]]; then
        "$file" "$@"
        ret=$?

        if [[ $ret -gt $FAILED ]]; then
            FAILED=$?
        fi
    fi
done

exit "$FAILED"
