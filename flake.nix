{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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

      circomspect = pkgs.rustPlatform.buildRustPackage rec {
        pname = "circomspect";
        version = "0.9.0";

        src = pkgs.fetchFromGitHub {
          owner = "trailofbits";
          repo = "circomspect";
          rev = "v${version}";
          hash = "sha256-rhWiTvFlQeFNoafmk891KR6Aj2qrm3v3csurNppTt68=";
        };

        cargoHash = "sha256-SNY/QFOUAOszvhCGORp4sTaseLSlzylduVsa68ytIOM=";
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

    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          pkgs.circom
          pkgs.just
          pkgs.nodejs
          pkgs.pnpm
          pkgs.foundry

          circomspect

          rustToolchain
          pkgs.bacon
          pkgs.wasm-pack
        ];
      };
    };
}
