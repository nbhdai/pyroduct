{ pkgs, craneLib, commonArgs, miriToolchain }:

let
  test-str = ''
    echo "Running Miri tests for vec_buf_safety..."
    cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test vec_buf_safety "$@"
    echo "Running Miri tests for log..."
    MIRIFLAGS=-Zmiri-disable-isolation cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test logs "$@"
    echo "Running Miri tests for ffi_safety..."
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows" cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test ffi_safety "$@"
    echo "Running Miri tests for wal..."
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows" cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test wal_safety "$@"
  '';

  test-miri = pkgs.writeShellScriptBin "test-miri" ''
    set -e
    export PATH="${miriToolchain}/bin:$PATH"
    export RUST_BACKTRACE=1
    ${test-str}
  '';
in
{
  check = craneLib.cargoBuild (commonArgs // {
    pname = "pyroduct-miri-tests";
    
    cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
      pname = "pyroduct-deps-miri";
    });

    doCheck = true;

    checkPhase = ''
      export RUST_BACKTRACE=1
      ${test-str}
    '';

    buildPhase = "true";
    installPhase = "mkdir -p $out";
  });

  bin = test-miri;
}
