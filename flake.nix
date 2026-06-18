{
  description = "Noir + Barretenberg Environment (1.0.0 Beta)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      # The exact matched compiler/prover pairs used in Aztec's Mainnet RC
      bbVersion = "5.0.0-nightly.20260522";
      noirVersion = "1.0.0-beta.22";

      pkgs = nixpkgs.legacyPackages.${system};

      barretenberg = pkgs.stdenv.mkDerivation {
        pname = "barretenberg";
        version = bbVersion;

        src = pkgs.fetchurl {
          # Aztec moved standalone binaries to this dedicated mirror repo
          # and renamed the artifact to amd64-linux
          url = "https://github.com/AztecProtocol/barretenberg/releases/download/v${bbVersion}/barretenberg-amd64-linux.tar.gz";
          hash = "sha256-0gfskPv6L7ok16R7enWJLuBSt5hCUrhmpKDBtSluFXE=";
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
        version = noirVersion;

        src = pkgs.fetchurl {
          url = "https://github.com/noir-lang/noir/releases/download/v${noirVersion}/nargo-x86_64-unknown-linux-gnu.tar.gz";
          hash = "sha256-OExPyACQWyE+Jqq9c4qWpKhbGnb/wn+xmuttM0lKeHs=";
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
          pkgs.circom
          pkgs.just
          pkgs.nodejs_22
          pkgs.pnpm
        ];
      };
    };
}
