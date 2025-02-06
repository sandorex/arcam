# release workflow
#
# 1. change version in Cargo.toml
# 2. run cargo build
# 3. git add Cargo.toml Cargo.lock
# 4. git commit -m 'chore: bump version to X.X.X'
# 5. just tag

default:
    @just --list

# get cargo package version
get-version:
    @cargo pkgid | perl -pe '($_)=/([0-9]+([.][0-9]+)+)/'

# run cargo check test and clippy
test:
    cargo check
    cargo test
    cargo clippy

# tag current package version in git
tag:
    #!/usr/bin/env bash
    set -e

    # test just in case
    echo "Make sure you ran all the tests"
    read -n1
    read -n1

    version="$(just get-version)"
    if [[ -z "$version" ]]; then
        echo "Could not read the cargo version"
        exit 1
    fi

    if [[ -n "$(git tag -l "v${version}")" ]]; then
        echo "Tag v${version} already exists, did you forget to update Cargo.toml?"
        exit 1
    fi

    if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
        echo "There are uncommited git changes"
        exit 1
    fi

    echo "Tagging v${version}"
    git tag "v${version}"
