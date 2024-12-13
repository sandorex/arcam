## Frequently Asked Questions

### Emacs TRAMP
To use emacs TRAMP with arcam add following to your `~/.emacs.d/init.el`
```elisp
(use-package tramp
  :config
  (setopt remote-file-name-inhibit-cache 30)
  (setopt tramp-verbose 1) ;; only log errors

  ;; do not use cache for completion as it keeps suggesting non-existing containers
  (setopt tramp-completion-use-cache nil)

  ;; use the PATH set inside the container
  (add-to-list 'tramp-remote-path 'tramp-own-remote-path)

  ;; add arcam support directly
  (add-to-list 'tramp-methods
               '("arcam"
                 (tramp-login-program "arcam")
                 (tramp-remote-shell "/bin/sh")
                 (tramp-login-args (("exec") ("%h") ("--") ("%l")))
                 (tramp-direct-async ("/bin/sh" "-c"))
                 (tramp-remote-shell-login ("-l"))
                 (tramp-remote-shell-args ("-i" "-c"))
                 ))

  (defun arcam--tramp-completion (&optional ignored)
    (when-let ((raw (shell-command-to-string "arcam list --raw"))
               (lines (split-string raw "\n" 'omit))
               (containers (mapcar (lambda (x) ; split by tab and map it
                                     (let ((split (split-string x "\t" 'omit)))
                                       `(nil ,(nth 0 split)))) lines)))
      containers))

  (tramp-set-completion-function "arcam" '((arcam--tramp-completion ""))))
```

Now you should be able to use new *protocol* `/arcam:<container-name>:` when you press `C-x C-f`

### Data Persistance
**NOTE: This also breaks sandboxing a bit if you share the volume between containers**

One of annoying side effects of ephemeral containers is the install at each startup, so to mitigate it with neovim plugins in this case you can use following:

Add a named volume as engine args
```
arcam start -- --volume data:/data
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
