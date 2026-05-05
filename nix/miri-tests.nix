{ pkgs, craneLib, commonArgs }:

craneLib.cargoBuild (commonArgs // {
  pname = "pyroduct-miri-tests";
  
  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    pname = "pyroduct-deps-miri";
  });

  doCheck = true;

  checkPhase = ''
    export RUST_BACKTRACE=1
    
    echo "Running Miri tests for vec_buf_safety..."
    cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test vec_buf_safety
    
    echo "Running Miri tests for ffi_safety..."
    MIRIFLAGS="-Zmiri-disable-isolation -Zmiri-tree-borrows" cargo miri test --manifest-path lib/pyroduct/Cargo.toml --all-features --test ffi_safety
  '';

  buildPhase = "true";
  installPhase = "mkdir -p $out";
})
