default:
    @just --list

# get cargo package version
get-version:
    @cargo pkgid | cut -d "@" -f2

# run cargo check test and clippy
test:
    cargo check
    cargo test
    cargo clippy

# tag current package version in git
git-tag:
    #!/usr/bin/env bash
    set -e

    version="$(just get-version)"
    if [[ -z "$version" ]]; then
        echo "Could not read the cargo version"
        exit 1
    fi

    if [[ -n "$(git tag -l "v${version}")" ]]; then
        echo "Tag v${version} already exists, did you forget to update Cargo.toml?"
        exit 1
    fi

    if [[ -n "$(git status --porcelain)" ]]; then
        echo "There are uncommited git changes"
        exit 1
    fi

    just test

    echo "Tagging v${version}"
    git tag "v${version}"
