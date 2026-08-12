{
  lib,
  python3Packages,
  fetchFromGitHub,
  z3,
}:

let
  rich14 = python3Packages.rich.overridePythonAttrs (old: rec {
    version = "14.0.0";
    src = python3Packages.fetchPypi {
      pname = "rich";
      inherit version;
      hash = "sha256-gvG8I6aiHrykrgxFr5vbxJLtICMdy2Pyl9bRAhqdVyU=";
    };
    doCheck = false;
  });
in
python3Packages.buildPythonApplication rec {
  pname = "halmos";
  version = "0.3.3";
  pyproject = true;

  src = fetchFromGitHub {
    owner = "a16z";
    repo = "halmos";
    rev = "v${version}";
    hash = "sha256-oYouyic9SZuPxpRT1yk8fVQQmU33Y13E3Qil2eLx0lw=";
    leaveDotGit = true;
  };

  SETUPTOOLS_SCM_PRETEND_VERSION = version;

  build-system = with python3Packages; [
    setuptools
    setuptools-scm
  ];

  dependencies = with python3Packages; [
    sortedcontainers
    toml
    z3-solver
    eth-hash
    pycryptodome
    rich14 # pinned
    xxhash
    psutil
    requests
    python-dotenv
  ];

  pythonRemoveDeps = [
    "z3-solver"
    "yices-solver"
  ];

  makeWrapperArgs = [
    "--prefix PATH : ${lib.makeBinPath [ z3 ]}"
  ];

  pythonImportsCheck = [ "halmos" ];

  meta = {
    description = "Symbolic testing tool for EVM smart contracts";
    homepage = "https://github.com/a16z/halmos";
    license = lib.licenses.agpl3Only;
    mainProgram = "halmos";
  };
}
