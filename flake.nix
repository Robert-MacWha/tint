{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      rustToolchain = pkgs.rust-bin.stable."1.93.0".default.override {
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

      # noir = pkgs.callPackage ./nix/noir/package.nix { };
      # barretenberg = pkgs.callPackage ./nix/barretenberg/package.nix { };
      # provekit = pkgs.callPackage ./nix/provekit/package.nix { };

    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          pkgs.just
          pkgs.pnpm
          pkgs.foundry

          rustToolchain
          pkgs.cargo-insta
          pkgs.bacon
          pkgs.wasm-pack

          pkgs.go
          pkgs.gopls

          # noir
          # barretenberg
          # provekit
        ];
      };
    };
}
