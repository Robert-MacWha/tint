{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};

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
        ];
      };
    };
}
