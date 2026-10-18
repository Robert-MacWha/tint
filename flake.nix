{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      nixpkgs-unstable,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      unstable = import nixpkgs-unstable {
        inherit system;
      };

      rustToolchain = pkgs.rust-bin.stable."1.98.0".default.override {
        extensions = [
          "rust-src"
          "llvm-tools"
          "rust-analyzer"
        ];
        targets = [
          "wasm32-unknown-unknown"
          "wasm32-wasip1"
        ];
      };

      rustfmtNightly = pkgs.rust-bin.nightly.latest.rustfmt;

      yices = pkgs.callPackage ./nix/yices/package.nix { };
      halmos = pkgs.callPackage ./nix/halmos/package.nix { };
      picus = pkgs.callPackage ./nix/picus/package.nix { };
      alto = pkgs.callPackage ./nix/alto/package.nix { };
    in
    {
      devShells.${system} = {
        default = pkgs.mkShell {
          buildInputs = [
            pkgs.just
            unstable.foundry
            yices
            halmos
            picus
            alto

            rustfmtNightly
            rustToolchain
            # required for bindgen
            # https://wiki.nixos.org/wiki/Rust#Installating_with_bindgen_support
            pkgs.rustPlatform.bindgenHook

            pkgs.cargo-autoinherit
            pkgs.cargo-machete
            pkgs.cargo-sort
            pkgs.cargo-insta
            pkgs.bacon
            pkgs.wasm-pack

            pkgs.go
            pkgs.gopls
          ];
        };

        ci = pkgs.mkShell {
          buildInputs = [
            pkgs.just
            unstable.foundry
            alto
            rustToolchain
            pkgs.rustPlatform.bindgenHook
            pkgs.go
          ];
        };
      };
    };
}
