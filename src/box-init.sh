#!/usr/bin/env bash
# init script for boxes, ran inside the container as the entrypoint

export BOX_VERSION='0.1'

# prevent running on bare host
[[ -v container ]] || exit 69

set -eu -o pipefail

# allow debugging on demand
if [[ -v BOX_DEBUG ]]; then
    set -x
fi

echo "box-init $BOX_VERSION"

# TODO experiment with getting last user in /etc/passwd so there is no need for env var
if [[ ! -v HOST_USER ]]; then
    echo "Container initialization requires host user"
    exit 1
fi

# you probably won't have fish and zsh installed and as bash is required, any
# other shell is considered as the default so
if [[ -f /bin/fish ]]; then
    shell=/bin/fish
elif [[ -f /bin/zsh ]]; then
    shell=/bin/zsh
else
    shell=/bin/bash
fi

echo "Setting the user home and shell"
usermod -d "/home/${HOST_USER:?}" -s "${BOX_SHELL:-$shell}" "${HOST_USER:?}"

echo "Setting up user home from /etc/skel"
/sbin/mkhomedir_helper "${HOST_USER:?}"

echo "Running /init.d/ scripts"
# run user scripts
if [[ -d /init.d ]]; then
    for script in /init.d/*; do
        if [[ -x "$script" ]]; then
            # run each script as user
            sudo -u "${HOST_USER:?}" "$script"
        fi
    done
fi

echo "Starting infinite loop (Ctrl + C to close)"

# make sure the container stays alive
sleep infinity &

# make container respond to being killed
on_sigterm() {
	echo Caught SIGTERM, exiting...
	jobs -p | xargs -r kill -TERM
	wait
}

trap "on_sigterm" TERM INT
wait
