## Frequently Asked Questions

### Data Persistance
**NOTE: This makes also breaks sandboxing a bit if you share the volume between containers**

One of annoying side effects of ephemeral containers is the install at each startup, so to mitigate it with neovim plugins in this case you can use following:

Add a named volume as engine args
```
box start -- --volume box-data:/data
```

And then setup the neovim in container init script `/init.d/90-neovim.sh`:
```sh
#!/usr/bin/env bash
# using a link to make it store data in persistant volume
mkdir -p "$HOME/.local/share"
ln -sf /data/nvim "$HOME/.local/share/nvim"

if [[ ! -d /data/nvim ]]; then
    sudo mkdir /data/nvim
    sudo chown "$USER:$USER" /data/nvim

    # this is specific to my configuration but you setup your own bootstrapping function inside neovim
    nvim --headless +Bootstrap +q
fi
```

### Execute Commands on Host System
**NOTE: This efectively makes sandboxing redundant so it's not recommended!**

To allow execution of command on host system use `--session-bus` option (or in config) and download host-spawn in the container from [here](https://github.com/1player/host-spawn/releases/latest)

You can do it in Containerfile like so
```
RUN wget https://github.com/1player/host-spawn/releases/latest/host-spawn-x86_x64 -O /usr/local/bin/host-spawn
```
