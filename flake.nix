{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
  };

  outputs = { self, nixpkgs, naersk, rust-overlay }:
    let
      inherit self;
      system = "x86_64-linux";

      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };

      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
      cargoTarget = "x86_64-unknown-linux-musl";

      naersk' = pkgs.callPackage naersk {
        cargo = toolchain;
        rustc = toolchain;
      };
    in
    {
      packages.${system}.default = naersk'.buildPackage {
        src = ./.;

        # build using default target
        CARGO_BUILD_TARGET = cargoTarget;

        # make vergen_git2 happy
        VERGEN_IDEMPOTENT = "1";
        VERGEN_GIT_SHA = if (self ? "rev") then (builtins.substring 0 7 self.rev) else "nix-dirty";
      };

      devShells.${system}.default = pkgs.mkShell {
        # buildInputs = [ nano cargo rustc rustfmt rust-analyzer pre-commit rustPackages.clippy ];
        nativeBuildInputs = with pkgs; [ git toolchain ];

        shellHook = ''
          echo "Development shell for arcam"

          alias build='cargo build'
          alias build-release='cargo build --release'
          alias test='cargo test'
        '';

        CARGO_BUILD_TARGET = "${cargoTarget}";
      };
    };
}
