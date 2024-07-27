#!/usr/bin/env bash
# neovim boostrap

set -x

# using a link to store plugins and stuff in the persistent data volume
mkdir -p "$HOME/.local/share"
ln -sf /data/nvim "$HOME/.local/share/nvim"

if [[ -d /data/nvim ]]; then
    echo "neovim cache exists doing nothing"
else
    echo "neovim cache does not exist, bootstrapping"

    sudo mkdir /data/nvim
    sudo chown "$USER:$USER" /data/nvim
    nvim --headless +Bootstrap +q
fi
