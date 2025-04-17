{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
  };

  outputs = { self, nixpkgs, naersk }:
     let
       inherit nixpkgs self;
       package = system: (nixpkgs.legacyPackages.${system}.callPackage naersk {}).buildPackage {
         src = ./.;

         # make vergen_git2 happy
         VERGEN_IDEMPOTENT = "1";
         VERGEN_GIT_SHA = if (self ? "rev") then (builtins.substring 0 7 self.rev) else "nix-dirty";
       };
       shell = system: with import nixpkgs { inherit system; }; mkShell {
            buildInputs = [ nano cargo rustc rustfmt rust-analyzer pre-commit rustPackages.clippy ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
       };
     in
     {
       packages.x86_64-linux.default = package "x86_64-linux";
       packages.aarch64-linux.default = package "aarch64-linux";

       devShells.x86_64-linux.default = shell "x86_64-linux";
       devShells.aarch64-linux.default = shell "aarch64-linux";

       # apple stuff (not officially supported)
       packages.x86_64-darwin.default = package "x86_64-darwin";
       packages.aarch64-darwin.default = package "aarch64-darwin";

       devShells.x86_64-darwin.default = shell "x86_64-linux";
       devShells.aarch64-darwin.default = shell "x86_64-linux";
     };
}
