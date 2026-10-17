# Packages Pimlico's `alto` ERC-4337 bundler (https://github.com/pimlicolabs/alto)
# from its published npm release.
{ pkgs }:

pkgs.buildNpmPackage rec {
  pname = "alto";
  version = "0.0.21";

  src = pkgs.fetchurl {
    url = "https://registry.npmjs.org/@pimlico/alto/-/alto-${version}.tgz";
    hash = "sha256-9BZ4c46YcA3ZKpDDL75/Snwx5r0oGhULHw+pjc84Pm4=";
  };

  sourceRoot = "package";

  # The published tarball doesn't ship an npm lockfile (uses pnpm instead).
  #
  # Since there isn't a well-supported buildPnpmPackage, just create and use an
  # npm lockfile. Generated once via `npm install --package-lock-only` against
  # this exact tarball and committed here. Regenerate it whenever `version`
  # is bumped.
  postPatch = ''
    cp ${./package-lock.json} package-lock.json
  '';

  npmDepsHash = "sha256-wzNf61f03OmroMUol6kFn6A+ZEWk88qBZl/ah0miD6E=";

  # Tarball already contains the prebuilt `esm/` output -- nothing to build.
  dontNpmBuild = true;

  meta = {
    description = "Pimlico's ERC-4337 bundler, packaged from the published npm release";
    homepage = "https://github.com/pimlicolabs/alto";
    license = pkgs.lib.licenses.gpl3Plus;
    mainProgram = "alto";
  };
}
