{
  description = "Dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      unstable,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        unstablePkgs = import unstable {
          inherit system;
        };

        rustToolchain = pkgs.rust-bin.stable."1.94.0".default.override {
          extensions = [
            "rust-src"
            "llvm-tools"
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        rustfmtNightly = pkgs.rust-bin.nightly.latest.rustfmt;

        noir = pkgs.callPackage ./pkgs/noir/package.nix { };
        barretenberg = pkgs.callPackage ./pkgs/barretenberg/package.nix { };
      in
      {
        devShells = {
          default = pkgs.mkShell {
            packages = [
              noir
              barretenberg
              pkgs.yarn-berry
            ];
          };
        };
      }
    );
}
