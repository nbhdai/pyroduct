{ craneLib, commonArgs }:

craneLib.cargoBuild (commonArgs // {
  pname = "pyroduct-rust-tests";

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    pname = "pyroduct-deps-rust-tests";
  });

  doCheck = true;
  
  checkPhase = ''
    export RUST_BACKTRACE=1
    cargo test --manifest-path lib/Cargo.toml --all-features
  '';

  buildPhase = "true";
  installPhase = "mkdir -p $out";
})
