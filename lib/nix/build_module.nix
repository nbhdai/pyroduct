{ pkgs, lib, craneLibWasm, toToml, mkDep, pyroductPath }:

{
  name,
  version ? "0.1.0",
  src,
  # REQUIRED: The generator needs to know where to write the Cargo.toml on disk
  srcPath ? "modules/${name}", 
  capabilities ? [],
  dependencies ? [],
  extraCargoToml ? {},
}: 

let
  formatDep = dep: 
    if builtins.isString dep then { name = dep; version = "*"; }
    else if builtins.isAttrs dep then dep
    else throw "Invalid dependency format";
  deps = map formatDep dependencies;

  # --- Generate Cargo.toml Content ---
  cargoToml = {
    package = {
      inherit name version;
      edition = "2024";
      authors = [ "Sven Cattell" ];
    };
    lib = { crate-type = [ "cdylib" ]; };
    dependencies = lib.listToAttrs (
      # Regular dependencies
      (map (d: { name = d.name; value = mkDep d; }) deps) ++
      # Capability dependencies
      (map (cap: { 
        name = cap.pname or cap.name; 
        value = mkDep {
          name = cap.pname or cap.name;
          # Point strictly to the 'crate' folder inside the capability artifact
          path = "${cap.output}/crate";
          features = [ "module" ];
        }; 
      }) capabilities)
    ) // {
      pyroduct = { path = pyroductPath; };
      tracing = "*";
    };
  } // extraCargoToml;

  cargoTomlContent = toToml cargoToml;

  # --- Build the Source with Cargo.toml ---
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

  # --- Compile WASM ---
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