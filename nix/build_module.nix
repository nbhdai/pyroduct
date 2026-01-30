{ pkgs, lib, craneLibWasm, makeModuleToml }:

args@{
  name,
  version ? "0.1.0",
  src,
  srcPath ? "modules/${name}", 
  capabilities ? [],
  ...
}: 

let
  # --- 1. Generate Cargo.toml Content ---
  cargoTomlContent = makeModuleToml args;

  # --- 2. Build the Source with Cargo.toml ---
  srcWithCargoToml = pkgs.runCommand "${name}-src" {} ''
    mkdir -p $out
    cp -r ${src}/* $out/ 2>/dev/null || true  
    cp -r ${src}/. $out/ 2>/dev/null || true
    chmod -R u+w $out
    cat > $out/Cargo.toml << 'CARGO_TOML_EOF'
${cargoTomlContent}
CARGO_TOML_EOF
  '';

in {
  inherit name version cargoTomlContent srcPath;
  capabilities = capabilities;
  src = srcWithCargoToml;
  
  # --- 3. Compile WASM ---
  wasm = craneLibWasm.buildPackage {
    pname = name;
    inherit version;
    src = srcWithCargoToml;
    cargoExtraArgs = "--target wasm32-unknown-unknown -p ${name}";
    doCheck = false;
    
    installPhase = ''
      mkdir -p $out/lib
      cp target/wasm32-unknown-unknown/release/*.wasm $out/lib/
    '';
  };
  
  # Helper to get the list of host binaries needed for this module
  hostPlugins = map (cap: cap.binaryPath) capabilities;
}