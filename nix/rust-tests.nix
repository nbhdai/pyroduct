{ pkgs, craneLib, commonArgs, pyroduct }:

let
  prepare = pkgs.writeShellScriptBin "prepare-pyro" ''
    pyroduct clean ./capabilities
    pyroduct ship ./capabilities -d
    pyroduct expand ./capabilities
    pyroduct clean ./modules
    pyroduct ship ./modules -d
    pyroduct expand ./modules
  '';

  test-rust = pkgs.writeShellScriptBin "test-rust" ''
    cargo test --manifest-path lib/Cargo.toml --all-features "$@"
  '';
in
{
  check = craneLib.cargoBuild (commonArgs // {
    pname = "pyroduct-rust-tests";

    cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
      pname = "pyroduct-deps-rust-tests";
    });

    doCheck = true;
    
    nativeBuildInputs = (commonArgs.nativeBuildInputs or []) ++ [ pyroduct test-rust ];

    checkPhase = ''
      export RUST_BACKTRACE=1
      pyroduct clean ./capabilities
      pyroduct ship ./capabilities -d
      pyroduct expand ./capabilities
      pyroduct clean ./modules
      pyroduct ship ./modules -d
      pyroduct expand ./modules
      test-rust
    '';

    buildPhase = "true";
    installPhase = "mkdir -p $out";
  });

  bin = test-rust;
  prepare = prepare;
}
