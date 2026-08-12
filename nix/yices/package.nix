{
  lib,
  stdenv,
  fetchFromGitHub,
  autoconf,
  gperf,
  gmp,
  which,
}:

stdenv.mkDerivation rec {
  pname = "yices";
  version = "2.6.4";

  src = fetchFromGitHub {
    owner = "SRI-CSL";
    repo = "yices2";
    rev = "Yices-${version}";
    hash = "sha256-qdxh86CkKdm65oHcRgaafTG9GUOoIgTDjeWmRofIpNE=";
  };

  nativeBuildInputs = [
    autoconf
    gperf
    which
  ];
  buildInputs = [ gmp ];

  preConfigure = "autoconf";

  installFlags = [ "LDCONFIG=true" ];

  enableParallelBuilding = true;

  meta = {
    description = "SMT solver from SRI International";
    homepage = "https://yices.csl.sri.com/";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.unix;
    mainProgram = "yices-smt2";
  };
}
