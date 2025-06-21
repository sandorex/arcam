## arcam
![Crates.io Version](https://img.shields.io/crates/v/arcam)
![GitHub Release](https://img.shields.io/github/v/release/sandorex/arcam)

Fast sandboxed development container manager using podman, minimal permissions by default choose balance between security and convenience

![Demo GIF](demo.gif)

### Features
- Sandboxed ephemeral container by default (podman defaults with network turned off by default)
- Pass through audio, wayland, ssh-agent easily on demand with flags or config
- TOML configuration files for containers, customize your experience per project requirements
- Override dotfiles locally, so you don't have to rebuild the image to update dotfiles
- Automatic passwordless sudo *(or `su` if `sudo` is not available)*
- Host terminfo integration, you do not have to install packages to use kitty, or wezterm
- Consistant development environment on any distro, especially useful on distros like fedora atomic
- Offline use, container initialization process does not require internet connection *(image has to be downloaded of course)*

### Installation
You can download binary for latest release [here](https://github.com/sandorex/arcam/releases/latest/download/arcam)

Alternatively you can install it from crates.io
```sh
cargo install arcam
```

You can also install straight from git
```
cargo install --git https://github.com/sandorex/arcam
```

<details>
<summary>Using Nix</summary>

#### Nix
You can run it in a shell like so
```
nix shell github:sandorex/arcam
```

Or install it into your profile
```
nix profile install github:sandorex/arcam
```

</details>

### Usage
To avoid out-of-date documentation probably all the help you'll need is included in the binary itself\
For help with config options run `arcam config --options`, or to see an example config run `arcam config --example`

<details>
<summary>Custom Container Images</summary>

### Custom Container Images
Making a custom container image is same as for any other container, to take full advantage of arcam keep following things in mind:
- Any file in `/init.d` will be executed on start of the container as the user, use `asroot` (wraps `su` or `sudo` if it exists) to run commands as root
- Put dotfiles in `/etc/skel` which will be copied to user home on start, note that it may be overriden at runtime using `--skel`
- All data inside the container (not counting volumes) will be deleted when container stops, to add caching or presistant data use a named volume

For examples you can take a look at [my everchanging containers](https://github.com/sandorex/config/tree/master/boxes)

</details>

### Comparison to Other Tools
#### Toolbox / Distrobox
Both are great at their job, to provide a seamless integration with the host but not sandboxing

Arcam provides sandboxed experience by default, and it's your job to choose where/when to sacrifice security for convenience, it's highly configurable

<details>
<summary>Development Notes</summary>

### Development Notes
These are notes for me or anyone else hacking on this

#### Development
I have made `toolchain.toml` contain everything needed, and flake devshell also works great so development on any machine should be a breeze

#### Demo GIF
Use asciinema in 80x30 terminal

The GIF was generated with following command
```
agg --theme monokai \
    --font-family 'FiraCode Nerd Font' \
    --font-size 16 \
    --last-frame-duration 5 \
    demo.cast demo.gif
```

</details>

