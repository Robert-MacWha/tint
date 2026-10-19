# The workspace's vendored dependencies, optionally with std's, for crates
# built with `-Z build-std`.
{
  lib,
  rustPlatform,
  symlinkJoin,
  writeTextDir,
  rustToolchain,
}:

{
  withStd ? false,
}:

symlinkJoin {
  # importCargoLock's config expects the vendor directory under this name.
  name = "cargo-vendor-dir";
  paths = [
    (rustPlatform.importCargoLock {
      lockFile = ../../packages/crates/Cargo.lock;
      allowBuiltinFetchGit = true;
    })
    # ark-hybrid-compression documents itself with its repository's README,
    # from outside its crate, so it resolves to the vendor directory's root.
    (writeTextDir "README.md" "")
  ]
  ++ lib.optional withStd (
    rustPlatform.importCargoLock {
      lockFile = "${rustToolchain}/lib/rustlib/src/rust/library/Cargo.lock";
    }
  );
}
