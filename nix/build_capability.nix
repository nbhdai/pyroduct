{ pkgs, lib, craneLibNative, toToml, mkDep }:

{
  name,
  version ? "0.1.0",
  src,
  # Optional: only needed if you want to generate a Cargo.toml back to disk for dev
  srcPath ? null,
  hostDependencies ? [],
  moduleDependencies ? [],
  sharedDependencies ? [],
  extraCargoToml ? {},
  pyroduct ? { workspace = "pyroduct"; }
  ...
}: 

let
  # --- Dependency Formatting ---
  formatDep = dep: 
    if builtins.isString dep then { name = dep; version = "*"; }
    else if builtins.isAttrs dep then dep
    else throw "Invalid dependency format";

  hostDeps = map formatDep hostDependencies;
  moduleDeps = map formatDep moduleDependencies;
  sharedDeps = map formatDep sharedDependencies;
  
  hostFeatureList = map (d: "dep:${d.name}") hostDeps;
  moduleFeatureList = map (d: "dep:${d.name}") moduleDeps;

  # --- 1. Generate Cargo.toml Content ---
  cargoToml = {
    package = {
      inherit name version;
      edition = "2024";
      authors = [ "Sven Cattell" ];
    };
    lib = { crate-type = [ "cdylib" "rlib" ]; };
    features = {
      default = [];
      capability = hostFeatureList;
      module = moduleFeatureList;
    };
    dependencies = lib.listToAttrs (
      (map (d: { name = d.name; value = mkDep d; }) sharedDeps) ++
      (map (d: { name = d.name; value = mkDep (d // { optional = true; }); }) hostDeps) ++
      (map (d: { name = d.name; value = mkDep (d // { optional = true; }); }) moduleDeps)
    ) // {
      # Pyroduct path is injected here. 
      # If pyroductPath is a store path, this becomes an absolute path in the generated toml.
      pyroduct = pyroduct;
    };
  } // extraCargoToml;

  cargoTomlContent = toToml cargoToml;

  # --- 2. Create the Source Tree with Generated Cargo.toml ---
  srcWithCargoToml = pkgs.runCommand "${name}-src" {} ''
    mkdir -p $out
    cp -r ${src}/* $out/ 2>/dev/null || true
    cp -r ${src}/. $out/ 2>/dev/null || true
    chmod -R u+w $out
    cat > $out/Cargo.toml << 'CARGO_TOML_EOF'
${cargoTomlContent}
CARGO_TOML_EOF
  '';

  # --- 3. Compile the Native Binary (Intermediate Step) ---
  compiledBinary = craneLibNative.buildPackage {
    pname = name;
    inherit version;
    src = srcWithCargoToml;
    cargoExtraArgs = "--features capability -p ${name}";
    cargoVendorDir = null;
    # We only care about the library output
    installPhase = ''
      mkdir -p $out/lib
      cp target/release/*.so $out/lib/ 2>/dev/null || \
      cp target/release/*.dylib $out/lib/ 2>/dev/null || \
      cp target/release/*.dll $out/lib/ 2>/dev/null || true
    '';
  };

  # --- 4. Generate Metadata/Docs ---
  docsJson = pkgs.writeText "docs.json" (builtins.toJSON {
    inherit name version;
    type = "capability";
    features = hostFeatureList ++ moduleFeatureList;
  });

  # --- 5. Assemble the Final Artifact ---
  # Structure:
  # /crate (source)
  # /libname.so (binary)
  # /docs.json
  artifact = pkgs.runCommand "${name}-artifact" {} ''
    mkdir -p $out/crate
    
    # Copy source code
    cp -r ${srcWithCargoToml}/* $out/crate/

    # Copy binary to root
    find ${compiledBinary}/lib -maxdepth 1 -type f \( -name "*.so" -o -name "*.dylib" -o -name "*.dll" \) -exec cp {} $out/ \;

    # Copy docs
    cp ${docsJson} $out/docs.json
  '';

  # Determine binary extension for helper path
  libExt = if pkgs.stdenv.isDarwin then "dylib" else "so";

in {
  inherit name version cargoTomlContent srcPath;
  pname = name;
  
  # The output is the full artifact folder in /nix/store
  output = artifact;
  
  # For convenience, pointers to specific parts
  src = "${artifact}/crate";
  binaryPath = "${artifact}/lib${name}.${libExt}";
  docsPath = "${artifact}/docs.json";
  
  # Kept for backward compatibility if needed
  hostPlugin = compiledBinary;
}