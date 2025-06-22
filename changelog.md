# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2025-06-22

### Features

- Pull images interactively if they don't exist ([0c227ac](https://github.com/sandorex/arcam/commit/0c227acd0b50ba49b4b7e774f1d2f643166efd99))
- Read system configs from /etc/arcam/configs ([b7b939b](https://github.com/sandorex/arcam/commit/b7b939b5284ba37ba2cf73f2a4667c40bff41afa))
- [**breaking**] Add --pulseaudio and --pipewire, remove --audio permission flags to start command ([25dca4f](https://github.com/sandorex/arcam/commit/25dca4f58fb26f46140200a40f9901e395bddf91))
- [**breaking**] Rework config to use version string to support future changes to config without breaking existing ([d435742](https://github.com/sandorex/arcam/commit/d43574287dd40b7f211a8718c4265956056a7485))
- Allow disabling container suffix by setting env var to empty string ([c8d93fd](https://github.com/sandorex/arcam/commit/c8d93fda09f37fa66c40429ec75b4460215b8669))
- [**breaking**] Change back to numbered config versions instead of calendar ([7a11153](https://github.com/sandorex/arcam/commit/7a11153dd02151d5881cc20c77dc6d2aded354f3))
- Add config version in the config --options command ([7dfff8f](https://github.com/sandorex/arcam/commit/7dfff8f954664bbf4813760e8d5d1efa901e8491))

### Bug Fixes

- Respect `TERMINFO_DIRS` when defined, and use `infocmp -D` to get terminfo directories if possible ([35b06ba](https://github.com/sandorex/arcam/commit/35b06ba108acd8ba9898c23933c23b113b352a24))

### Documentation

- Improve readme readability, hide development notes, add nix section ([1e0323c](https://github.com/sandorex/arcam/commit/1e0323caa17ae74278936e604ad17a843bd91972))

### Testing

- Fix tests failing in CI cause of interactive image pulling ([f06e496](https://github.com/sandorex/arcam/commit/f06e4966af9da943318a5730fb9ab1bb9762bf3d) [0bcb5c6](https://github.com/sandorex/arcam/commit/0bcb5c6d66f72982c0a71d4c524500fe4dd8a550) [ec9d6bc](https://github.com/sandorex/arcam/commit/ec9d6bcb5a0cf9b3aa5d59dbe0a98a2cb2b1d5fb))
- Add test case for start command output, to ensure the output is proper ([407ae72](https://github.com/sandorex/arcam/commit/407ae72acad3a28a9b8e2939236ee037628135a2))
- Add auto publish to crates in release workflow ([7bf69ee](https://github.com/sandorex/arcam/commit/7bf69eef7f3e929e7636ee7f781c9c51dec3de54))

### Nix

- Add nix flake with devshell and package with vergen data being set properly ([6940bd7](https://github.com/sandorex/arcam/commit/adf34f54d7faa60674ac1da4345641c60ea38179))

### Miscellaneous Tasks

- Removing mentions of docker and cleaning up some code ([c0e60aa](https://github.com/sandorex/arcam/commit/c0e60aaaac1d22e6ebeb682365a4dec6bd892d81))
- Improve errors and logging category for commands ran ([ee17d1f](https://github.com/sandorex/arcam/commit/ee17d1fdf43edaacac6d15e3c0953c6613da1c2d))
- Make --version be more verbose with more information ([671f9f7](https://github.com/sandorex/arcam/commit/671f9f7c9fd587b67ff7911400c568427fdc87b9))
- Make all commands log result as debug ([5506495](https://github.com/sandorex/arcam/commit/550649518350f5a834b474cd7e772b5eed6ddcae))
- Update cargo dependencies ([9c8d886](https://github.com/sandorex/arcam/commit/9c8d886fb66c79843e7ffc43c9161439ec6a2043))

## [0.1.12] - 2025-02-06

**This release is just re-release of v0.1.11 cause of a mistake during publishing**

For changelog go to [release notes of 0.1.11](https://github.com/sandorex/arcam/releases/tag/v0.1.11)

## [0.1.11] - 2025-02-05

### 🚀 Features

- Improve config options to be more readable and show an example ([5b069e8](https://github.com/sandorex/arcam/commit/5b069e853e78b6227583a83300c8ce89a37b8f77))
- Add host_pre_init config option to run commands on host before container ([f93358c](https://github.com/sandorex/arcam/commit/f93358cc0e6f9e0c95c2e14894de8f0afcc930fe))
- Add env var ARCAM_WAYLAND_DISPLAY to specify the display ([fcd1b38](https://github.com/sandorex/arcam/commit/fcd1b38ce0f8133919dd813a5c21a251c5edd5db))
- [**breaking**] Hide --engine flag as its useless ([e63ec56](https://github.com/sandorex/arcam/commit/e63ec5635ba0ec405561144925d7cb1d0dcf7b30))
- Create user group properly in the container ([cef710e](https://github.com/sandorex/arcam/commit/cef710e88521dfc42cd9dcf6e12ffa467b2b2b3a))
- Pass through fonts from host when passing through wayland socket ([ddaec79](https://github.com/sandorex/arcam/commit/ddaec790a6bedf4f59ded674aff11a391eb180f6))
- Add --raw and --here flags to list command for use in scripts ([dbf2511](https://github.com/sandorex/arcam/commit/dbf25119b7ac4084bd016ba41c7b01c6a04f485f))
- Better root process detection using /proc/x/stat instead of pgrep ([52ae6a0](https://github.com/sandorex/arcam/commit/52ae6a0e31e8eeb1e3033e6346231527840a701a))
- Flag to automatically kill container when no processes is running ([2e238e4](https://github.com/sandorex/arcam/commit/2e238e48d7eaa65fcea34e23f2b323a56adceb75))
- Add --login flag to exec to execute commands in login shell ([75f43e1](https://github.com/sandorex/arcam/commit/75f43e184b3cdeb45316d6ad8a497510954c8777))
- If shell command was killed do not print error message ([32faed5](https://github.com/sandorex/arcam/commit/32faed5caf799acb46c4896ef29ac78b81e2723c))
- Exec each shell from login /bin/sh ([3b087f0](https://github.com/sandorex/arcam/commit/3b087f0fc473baec55e37da2f9193a2846f856d1))
- [**breaking**] Config is reworked to be one configuration per file
    - host_pre_init, `on_init_pre`, `on_init_post` are strings now instead of arrays
- [**breaking**] Removed `--autoshutdown` as it was getting too complex and unreliable
- [**breaking**] `/shell` symlink removed and added argument `--shell` and `shell` in config
- [**breaking**] `config` command is reworked completely, with added `--options` and `--examples` to show them respectively
- Added `persist`/`persist_user` to replace volume and chown combo that is common
- Added autocompletion generation and basic helper function for writing them
- You can now use a config file path directly to start a container
- Added `asroot` script which runs command as root with `sudo` or `su`, prefers `sudo` if available

### 🐛 Bug Fixes

- File that should have been commited in last commit ([55b7c21](https://github.com/sandorex/arcam/commit/55b7c21962a5a5dbaaa44de024b416023d6e10c2))
- On_init_pre/_post writing and executing only the last line ([949e9b5](https://github.com/sandorex/arcam/commit/949e9b51d94969d9c312999f54333b6e2b771137))
- /init.d was not created automatically causing init to fail with --on-init ([42c5e6f](https://github.com/sandorex/arcam/commit/42c5e6fea1bed152f52f75899db36b9d9c9243f4))
- Ensure all init scripts execute in correct order by their name ([da3f621](https://github.com/sandorex/arcam/commit/da3f62141b59aae1928476c9fe84f10caf599d54))
- Remove requirement to run tests when tagging ([1bf3f05](https://github.com/sandorex/arcam/commit/1bf3f0536a4aadce01f965c610668916ea875060))
- Fix justfile typo ([9b3dd0c](https://github.com/sandorex/arcam/commit/9b3dd0c688b97659ce263e82a79b876e3a2ea3a0))

### 🚜 Refactor

- A lot of cleanup and rework with anyhow ([c14a5ca](https://github.com/sandorex/arcam/commit/c14a5ca0abece1dcee5f61db4220c5934d5614b5))
- Now generating on_init scripts with `set -e` for better reliability
- Updated README and new gif with its cast file
- Replaced expect tests with rust code with `rexpect` and `assert_cmd` crates
- Added grouping for `start` command help page

### 📚 Documentation

- Remove outdated info in README.md ([8d49e30](https://github.com/sandorex/arcam/commit/8d49e30d2eefd0a34f8b36cf71a64f3952ade8c1))
- Improve README.md, fix some typos ([d36f5eb](https://github.com/sandorex/arcam/commit/d36f5eb40dffa84ba7a66338cc20aa78628f7316))
- Add tramp method for arcam to FAQ ([8fcad2a](https://github.com/sandorex/arcam/commit/8fcad2aa72da5f1acacd74691dae2912d94fda0a))
- Add badges to readme ([d4a8c2e](https://github.com/sandorex/arcam/commit/d4a8c2eb1460f4e06b16c253ff3742b406bedd27))

### ⚙️ Miscellaneous Tasks

- Define all environment vars in single file as constants ([5b5bd7c](https://github.com/sandorex/arcam/commit/5b5bd7c76a7b89ad30a6ce9db84a766a72eb0458))
- Bump version to 0.1.11 ([f9ebd26](https://github.com/sandorex/arcam/commit/f9ebd26e6c4a15906f0c0f304748c0dd1735ed84))
