{ pkgs, lib, craneLibNative, makeCapabilityToml }:

args@{
  name,
  version ? "0.1.0",
  src,
  srcPath ? null,
  ...
}: 

let
  # --- 1. Generate Cargo.toml Content using the helper ---
  # Pass all args through; the helper picks what it needs (dependencies, etc.)
  cargoTomlContent = makeCapabilityToml args;

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

  # --- 3. Compile the Native Binary ---
  compiledBinary = craneLibNative.buildPackage {
    pname = name;
    inherit version;
    src = srcWithCargoToml;
    cargoExtraArgs = "--features host -p ${name}";
    
    installPhase = ''
      mkdir -p $out/lib
      cp target/release/*.so $out/lib/ 2>/dev/null || \
      cp target/release/*.dylib $out/lib/ 2>/dev/null || \
      cp target/release/*.dll $out/lib/ 2>/dev/null || true
    '';
  };

  # --- 4. Generate Metadata/Docs ---
  # Note: We reconstruct feature list slightly redundantly here or we could return it from toml gen
  # For now, simplistic recreation is fine:
  hostDependencies = args.hostDependencies or [];
  moduleDependencies = args.moduleDependencies or [];
  
  formatDepName = d: if builtins.isString d then d else d.name;
  hostFeatureList = map (d: "dep:${formatDepName d}") hostDependencies;
  moduleFeatureList = map (d: "dep:${formatDepName d}") moduleDependencies;

  docsJson = pkgs.writeText "docs.json" (builtins.toJSON {
    inherit name version;
    type = "capability";
    features = hostFeatureList ++ moduleFeatureList;
  });

  # --- 5. Assemble the Final Artifact ---
  artifact = pkgs.runCommand "${name}-artifact" {} ''
    mkdir -p $out/crate
    
    # Copy source code
    cp -r ${srcWithCargoToml}/* $out/crate/

    # Copy binary to root
    find ${compiledBinary}/lib -maxdepth 1 -type f \( -name "*.so" -o -name "*.dylib" -o -name "*.dll" \) -exec cp {} $out/ \;

    # Copy docs
    cp ${docsJson} $out/docs.json
  '';

  libExt = if pkgs.stdenv.isDarwin then "dylib" else "so";

in {
  inherit name version cargoTomlContent srcPath;
  pname = name;
  output = artifact;
  
  src = "${artifact}/crate";
  binaryPath = "${artifact}/lib${name}.${libExt}";
  docsPath = "${artifact}/docs.json";
  
  hostPlugin = compiledBinary;
}