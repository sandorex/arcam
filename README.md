## arcam
Fast sandboxed development container manager using podman, you poke holes in the sandboxed, minimal permissions by default

![Demo GIF](arcam-demo.gif)

**NOTE: Version 0.1.X is considered alpha and may break compatability at any time**

Experimental but all code since `v0.1.1` was written within a box container

*Originally named `box`, renamed to `arcam`*

### Features
- Sandboxed ephemeral container by default (podman defaults with network turned on)
- Pass through audio, wayland, ssh-agent easily on demand with flags or config
- Customize your experience per language, even per project
- Switch machines often? Just make the CI build your container with your dotfiles preincluded!
- Use different local dotfiles on demand (so you don't have to rebuild your container to update dotfiles)
- Automatic passwordless sudo (or `su` if not installed)

## Planned Features
These are features that are planned but the details are debatable

- Provide partial support for devcontainer.json
- Provide support for devcontainer features
- Partial docker support (i do not know if im able to support both docker and podman)

### Installation
You can download binary for latest release [here](https://github.com/sandorex/arcam/releases/latest/download/arcam)

Alternatively you can install it using cargo
```sh
cargo install --git https://github.com/sandorex/arcam
```

### Custom Container Image
Making a custom container image is same as for any other container, to take full advantage of box keep following things in mind:
- Install `sudo` for nicer experience
- Any executable files in `/init.d` will be executed on start of the container as the user, you can use `sudo` (may not be installed in some images) or `su` for root access
- Put dotfiles in `/etc/skel` which will be copied to user home on start, note that it will not be used if `--dotfiles` flag is used
- All data inside the container (not counting mounts) will be deleted when container stops, to add caching or presistant data use a named volume

For inspiration, or just an example take a look at [my container](https://github.com/sandorex/config/tree/master/boxes) with all languages i use and neovim preinstalled with my dotfiles

### Comparison to Other Tools
#### Toolbox / Distrobox
Both are great at their job, to provide a seamless integration with the host but not sandboxing

Box provides sandbox by default approach where you choose where to sacrifice sandboxing for convenience
