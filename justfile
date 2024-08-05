default:
    @just --list

get-version:
    @cargo pkgid | cut -d "@" -f2

test:
    cargo check
    cargo test
    cargo clippy

tag $version:
    #!/usr/bin/env bash
    set -e

    if [[ "$(just get-version)" != "$version" ]]; then
        echo "Version does not match cargo package!"
        #exit 1
    fi

    if [[ -n "$(git tag -l "v${version}")" ]]; then
        echo "Tag v${version} already exists!"
        exit 1
    fi

    just test

    echo git tag "v${version}"
