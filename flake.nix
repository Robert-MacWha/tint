{
  description = "Noir + Barretenberg Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

      barretenberg = pkgs.stdenv.mkDerivation {
        pname = "barretenberg";
        version = "0.63.0";

        src = pkgs.fetchurl {
          url = "https://github.com/AztecProtocol/aztec-packages/releases/download/aztec-packages-v0.63.0/barretenberg-x86_64-linux-gnu.tar.gz";
          hash = "sha256-Y9QUF5cRAZBcwhoHevA1hBJGmoGVQRDlalpraS92znE=";
        };

        nativeBuildInputs = [ pkgs.autoPatchelfHook ];

        buildInputs = [
          pkgs.stdenv.cc.cc.lib # libstdc++
          pkgs.libcxx # libc++
          pkgs.glibc # libc / libm
        ];

        sourceRoot = ".";

        installPhase = ''
          mkdir -p $out/bin
          cp bb $out/bin/
        '';
      };

      nargo = pkgs.stdenv.mkDerivation {
        pname = "nargo";
        version = "0.39.0";

        src = pkgs.fetchurl {
          url = "https://github.com/noir-lang/noir/releases/download/v0.39.0/nargo-x86_64-unknown-linux-gnu.tar.gz";
          hash = "sha256-E1OpSvYNouAtHuPSg74Q4DdBIfER0oLCR16cuXGyiBk=";
        };

        sourceRoot = ".";

        installPhase = ''
          mkdir -p $out/bin
          cp nargo $out/bin/
        '';
      };

    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          nargo
          barretenberg
        ];
      };
    };
}
