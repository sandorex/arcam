## box
Fast pet container manager, designed for sandboxed dev environment using podman *(docker is currently not supported but will be in the future)*

Currently experimental but i am using them personally

### Installation
Currently the easiest way to install it is using cargo, do not worry the binary builds quickly
```sh
cargo install --git https://github.com/sandorex/box
```

If you dont have cargo installed you can use a container to build it too
```sh
git clone https://github.com/sandorex/box
cd box
podman run --rm --user "$(id -u)":"$(id -g)" -v "$PWD":/usr/src/ws -w /usr/src/ws rust:latest cargo build --release
```

Github actions releases will come soon too

### Why Rust?
I wanted a single binary that could be distributed easily, even kept inside the container for easier installation

Current binary as of time of writing (30-07-2024) is around 1 megabyte, so im pretty happy with it

Original prototype was written in bash and while it was fine i did not feel like maintaining a growing complicated bash script split into multiple files

### Comparison to Other Tools
#### Toolbox / Distrobox
Both are quite good at what they do, providing integrated experience with the host, but the goal of this project is to provide a opposite experience a sandbox! While you can poke holes in it if you want, the default experience is pretty barebones

And no, you cannot make toolbox or distrobox sandboxed (at least at the moment of writing), i personally have nothing against either of them and use them myself but i always wanted a simple-ish system for specialized images to be used in sandboxed environment

### The Goal
Hackable "framework" to create your comfy containerized workflow, be it fully airtight container with no network, or container with RW access of your Home directory and SSH keys, **you choose the balance between security and comfort**

#### How Will It Work?
Well it kinda works already, but i plan to write documentation how to do specific things, customizations and such so you can build your own perfect environment

As writing all the tooling to change containers at runtime is plain stupid, i will focus on building customized container images

### Examples
- My own containers can be found in my [Dotfiles](https://github.com/sandorex/config) under [boxes](https://github.com/sandorex/config/tree/master/boxes)

