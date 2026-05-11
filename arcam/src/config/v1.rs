//! Configuration version 1.1

use code_docs::{DocumentedEnum, DocumentedStruct, code_docs_enum, code_docs_struct};
use serde::{Deserialize, Serialize};
use std::{fmt::{Display, Write}, path::PathBuf};

code_docs_enum! {
    #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
    pub enum SourceV1 {
        /// Use OCI image for the container source
        #[serde(rename = "image")]
        Image(String),

        /// Nix file or flake directory with specifier
        #[serde(rename = "nix")]
        Nix(PathBuf, Option<String>),

        // /// Use path as rootfs
        // Rootfs(PathBuf),
    }
}

impl Default for SourceV1 {
    fn default() -> Self {
        // this is just to implement default
        Self::Image("".to_owned())
    }
}

impl Display for SourceV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Image(image) => write!(f, "Image {image:?}")?,
            Self::Nix(path, spec) => if let Some(spec) = spec {
                write!(f, "Nix Flake {}#{spec}", path.display())?;
            } else {
                write!(f, "Nix File {}", path.display())?;
            },
        }

        Ok(())
    }
}

// save all the fields and docs so they can be printed as always up-to-date documentation
code_docs_struct! {
    /// Single configuration for a container, contains default settings and optional settings per
    /// engine that get applied over the default settings
    #[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct ConfigV1 {
        /// Path to the config
        /// @skip
        #[serde(skip)]
        pub path: Option<PathBuf>,

        /// Name of the config
        /// @skip
        #[serde(skip)]
        pub name: Option<String>,

        /// Source for the container
        /// @skip
        #[serde(flatten)]
        pub source: SourceV1,

        // --- real config options --- //

        /// Optional path to directory to use as /etc/skel (static dotfiles)
        ///
        /// Environ vars are expanded
        pub skel: Option<String>,

        /// Default user shell
        #[serde(default)]
        pub shell: Option<String>,

        /// Use gvisor runtime for better sandboxing
        #[serde(default)]
        pub gvisor: bool,

        /// Set network access
        #[serde(default)]
        pub network: bool,

        /// Passthrough pipewire
        #[serde(default)]
        pub pipewire: bool,

        /// Passthrough pulseaudio
        #[serde(default)]
        pub pulseaudio: bool,

        /// Passthrough wayland compositor socket, high security impact, allows clipboard access
        #[serde(default)]
        pub wayland: bool,

        /// Passthrough GPUs by their index, use 0 for all
        #[serde(default)]
        pub gpus: Vec<u8>,

        /// Passthrough ssh-agent socket, security impact is unknown
        #[serde(default)]
        pub ssh_agent: bool,

        /// Passthrough D-BUS session bus, maximum security impact allows arbitrary code execution
        #[serde(default)]
        pub session_bus: bool,

        /// Path to mount as a volume, basically shorthand for `--volume=<name>:<path>`
        #[serde(default)]
        pub persist: Vec<(String, String)>,

        /// Same as `persist` but the path is chowned as user on init
        #[serde(default)]
        pub persist_user: Vec<(String, String)>,

        /// Run command before all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_pre: Option<String>,

        /// Run command after all other scripts (ran using `/bin/sh`)
        #[serde(default)]
        pub on_init_post: Option<String>,

        /// Runs following shell script and pass all arguments verbatim to it,
        /// script itself is responsible for running arcam start with all the arguments
        ///
        /// This allows you total control of the container startup which also
        /// makes it dangerous if you do not check the config file beforehand
        ///
        /// NOTE: the script is ran using "/bin/sh"
        pub host_pre_init: Option<String>,

        /// Pass through container port to host (both TCP and UDP)
        ///
        /// Not all ports are allowed with rootless podman
        #[serde(default)]
        pub ports: Vec<(u32, u32)>,

        /// Environment variables to set (name, value)
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub env: Vec<(String, String)>,

        /// Add capabilities, or drop them with by prefixing `!cap`
        ///
        /// For more details about capabilities read `man 7 capabilities`
        #[serde(default)]
        pub capabilities: Vec<String>,

        /// Args passed to the engine
        ///
        /// Environ vars are expanded
        #[serde(default)]
        pub engine_args: Vec<String>,
    }
}

impl ConfigV1 {
    pub const VERSION: u32 = 1;

    /// Returns docs for the version of config
    pub fn docs() -> String {
        use std::fmt::Write;

        let mut output = String::new();

        // print config version in same style as the rest of options
        let _ = writeln!(
            &mut output,
            "/// Config schema version (latest: \"{}\")\nversion: String\n",
            Self::VERSION
        );

        // convert some types to be easier to understand for non-rust users
        let convert_type = |x: &str| -> String { x.replace("Vec<", "Array<") };

        // add properties from Source
        let iter = SourceV1::variant_names()
            .into_iter()
            .zip(SourceV1::variant_docs().into_iter());

        for (name, docs) in iter {
            for i in docs {
                let _ = writeln!(&mut output, "///{i}");
            }

            let _ = writeln!(&mut output, "{}: String\n", name.to_lowercase());
        }

        // add properties from Config
        let iter = Self::field_names()
            .into_iter()
            .zip(Self::field_types().into_iter())
            .zip(Self::field_docs().into_iter())
            .map(|((name, r#type), docs)| (name, r#type, docs));

        for (name, t, docs) in iter {
            // skip any that contains '@skip' in its docs
            if docs.join("\n").contains("@skip") {
                continue;
            }

            // format like rust docs
            for i in docs {
                let _ = writeln!(&mut output, "///{i}");
            }

            let _ = writeln!(&mut output, "{name}: {}\n", convert_type(t));
        }

        output.trim().to_owned()
    }

    /// Expands vars in select properties
    pub fn expand_vars<T: Fn(&str) -> Option<String>>(&mut self, getter: T) -> anyhow::Result<()> {
        use crate::util::expand_vars as expand;

        // expand engine args
        for arg in self.engine_args.iter_mut() {
            *arg = expand(arg, &getter)?;
        }

        // expand skel path
        if let Some(skel) = self.skel.as_mut() {
            *skel = expand(skel, &getter)?;
        }

        // expand only the values in env
        for (_, val) in self.env.iter_mut() {
            *val = expand(val, &getter)?;
        }

        Ok(())
    }

    // TODO this should probably be a trait so all config version could share this
    /// Merges cli into the config
    pub fn merge_cli(&mut self, args: &crate::cli::CmdStartArgs) -> &mut Self {
        if let Some(shell) = args.shell.as_ref() { self.shell = Some(shell.clone()); }
        if let Some(gvisor) = args.gvisor.as_ref() { self.gvisor = *gvisor; }
        if let Some(network) = args.network.as_ref() { self.network = *network; }
        if let Some(pipewire) = args.pipewire.as_ref() { self.pipewire = *pipewire; }
        if let Some(pulseaudio) = args.pulseaudio.as_ref() { self.pulseaudio = *pulseaudio; }
        if let Some(wayland) = args.wayland.as_ref() { self.wayland = *wayland; }
        if let Some(gpus) = args.gpus.as_ref() { self.gpus = gpus.clone(); }
        if let Some(ssh_agent) = args.ssh_agent.as_ref() { self.ssh_agent = *ssh_agent; }
        if let Some(session_bus) = args.session_bus.as_ref() { self.session_bus = *session_bus; }

        self.ports.extend_from_slice(&args.ports);
        self.capabilities.extend_from_slice(&args.capabilities);

        // append on init to the config value
        let mut on_init_pre = self.on_init_pre.take().unwrap_or_default();
        on_init_pre += &args.on_init_pre.join("\n");
        self.on_init_pre = Some(on_init_pre);

        let mut on_init_post = self.on_init_post.take().unwrap_or_default();
        on_init_post += &args.on_init_post.join("\n");
        self.on_init_post = Some(on_init_post);

        self
    }
}
