# Core library for building modules and capabilities
{ lib, pkgs, craneLibNative, craneLibWasm, workspaceRoot, pyroductPath }:

let
  # Helper to convert a Nix attrset to TOML format
  toToml = import ./to-toml.nix { inherit lib; };

  # Generate a dependency entry for Cargo.toml
  mkDep = { name, version ? null, path ? null, optional ? false, features ? [], ... }:
    let
      base = {}
        // (if path != null then { inherit path; } else {})
        // (if version != null then { inherit version; } else {});
      withOptional = if optional then base // { optional = true; } else base;
      withFeatures = if features != [] then withOptional // { inherit features; } else withOptional;
    in 
      # If only version, return just the version string
      if withFeatures == { version = version; } then version
      else withFeatures;

  # Generate feature flags for capability selection
  mkCapabilityFeatures = capabilities: 
    lib.listToAttrs (map (cap: {
      name = cap.pname;
      value = [ "dep:${cap.pname}" ];
    }) capabilities);

  # Build a capability - generates Cargo.toml with proper feature gates
  buildCapability = {
    name,
    version ? "0.1.0",
    src,
    # Path relative to workspace root for Cargo.toml generation
    srcPath ? null,
    # Dependencies only needed on host side (native plugin)
    hostDependencies ? [],
    # Dependencies only needed on module side (wasm)
    moduleDependencies ? [],
    # Dependencies needed on both sides
    sharedDependencies ? [],
    # Extra Cargo.toml sections
    extraCargoToml ? {},
  }: let
    # Convert dependency specs to Cargo.toml format
    formatDep = dep: 
      if builtins.isString dep then { name = dep; version = "*"; }
      else if builtins.isAttrs dep then dep
      else throw "Invalid dependency format";

    hostDeps = map formatDep hostDependencies;
    moduleDeps = map formatDep moduleDependencies;
    sharedDeps = map formatDep sharedDependencies;

    # Build the features section
    hostFeatureList = map (d: "dep:${d.name}") hostDeps;
    moduleFeatureList = map (d: "dep:${d.name}") moduleDeps;

    # Generate the Cargo.toml content
    cargoToml = {
      package = {
        inherit name version;
        edition = "2024";
        authors = [ "Sven Cattell" ];
      };

      lib = {
        crate-type = [ "cdylib" "rlib" ];
      };

      features = {
        default = [];
        capability = hostFeatureList;
        module = moduleFeatureList;
      };

      dependencies = lib.listToAttrs (
        # Shared dependencies (always included)
        (map (d: { 
          name = d.name; 
          value = mkDep d; 
        }) sharedDeps)
        ++
        # Host-only dependencies (optional, enabled by "host" feature)
        (map (d: { 
          name = d.name; 
          value = mkDep (d // { optional = true; }); 
        }) hostDeps)
        ++
        # Module-only dependencies (optional, enabled by "module" feature)
        (map (d: { 
          name = d.name; 
          value = mkDep (d // { optional = true; }); 
        }) moduleDeps)
      ) // {
        # Always depend on pyroduct
        pyroduct = { path = pyroductPath; };
      };
    } // extraCargoToml;

    cargoTomlContent = toToml cargoToml;

    # Create a derivation that includes the generated Cargo.toml
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
    inherit name version cargoTomlContent;
    pname = name;
    src = srcWithCargoToml;
    # Store the source path for Cargo.toml generation script
    srcPath = if srcPath != null then srcPath 
              else "proto/capabilities/${builtins.replaceStrings ["proto_"] [""] name}";
    
    # Build the host plugin (native shared library)
    hostPlugin = craneLibNative.buildPackage {
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

    # For use in module compilation (returns the path string for Cargo.toml)
    moduleSource = srcWithCargoToml;
    modulePath = "../../capabilities/${builtins.replaceStrings ["proto_"] [""] name}";
  };

  # Build a module - generates Cargo.toml with capability dependencies
  buildModule = {
    name,
    version ? "0.1.0",
    src,
    # Path relative to workspace root for Cargo.toml generation
    srcPath ? null,
    # List of capabilities (already built capability attrsets)
    capabilities ? [],
    # Regular dependencies
    dependencies ? [],
    # Extra Cargo.toml sections
    extraCargoToml ? {},
  }: let
    formatDep = dep: 
      if builtins.isString dep then { name = dep; version = "*"; }
      else if builtins.isAttrs dep then dep
      else throw "Invalid dependency format";

    deps = map formatDep dependencies;

    # Capability feature names
    capFeatureNames = map (cap: cap.pname or cap.name) capabilities;

    # Generate the Cargo.toml content
    cargoToml = {
      package = {
        inherit name version;
        edition = "2024";
        authors = [ "Sven Cattell" ];
      };

      lib = {
        crate-type = [ "cdylib" ];
      };

      dependencies = lib.listToAttrs (
        # Regular dependencies
        (map (d: { 
          name = d.name; 
          value = mkDep d; 
        }) deps)
        ++
        # Capability dependencies (optional, feature-gated)
        (map (cap: { 
          name = cap.pname or cap.name; 
          value = mkDep {
            name = cap.pname or cap.name;
            path = cap.modulePath or "../../capabilities/${builtins.replaceStrings ["proto_"] [""] (cap.pname or cap.name)}";
            features = [ "module" ];
          }; 
        }) capabilities)
      ) // {
        # Always depend on pyroduct and tracing
        pyroduct = { path = pyroductPath; };
        tracing = "*";
      };
    } // extraCargoToml;

    cargoTomlContent = toToml cargoToml;

    # Create a derivation that includes the generated Cargo.toml
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
    inherit name version cargoTomlContent;
    # Include the full capability objects for use in config generation
    capabilities = capabilities;
    src = srcWithCargoToml;
    # Store the source path for Cargo.toml generation script
    srcPath = if srcPath != null then srcPath
              else "proto/modules/${builtins.replaceStrings ["proto_" "module"] ["" "module"] name}";
    
    # Build the WASM module
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

    # Get the list of host plugins needed for this module
    hostPlugins = map (cap: cap.hostPlugin or null) capabilities;
  };

  # Header comment for generated Cargo.toml files
  generatedHeader = ''
# ============================================================================
# THIS FILE IS AUTO-GENERATED BY NIX - DO NOT EDIT MANUALLY
# ============================================================================
# This Cargo.toml was generated from the adjacent .nix file.
# Any manual changes will be overwritten on the next generation.
#
# To regenerate all Cargo.toml files, run:
#   nix run .#generate-cargo-toml
#
# To modify dependencies or configuration, edit the corresponding:
#   - capability.nix (for capabilities)
#   - module.nix (for modules)
# ============================================================================

'';

  # Add the header to a Cargo.toml content string
  withHeader = content: generatedHeader + content;

  # Generate a script that writes all Cargo.toml files to disk
  # Takes a list of { path, content } attrsets
  mkGenerateCargoTomlScript = items: pkgs.writeShellScriptBin "generate-cargo-toml" ''
    set -euo pipefail
    
    echo "Generating Cargo.toml files..."
    
    ${lib.concatMapStringsSep "\n" (item: ''
      echo "  Writing ${item.path}/Cargo.toml"
      mkdir -p "${item.path}"
      cat > "${item.path}/Cargo.toml" << 'CARGO_TOML_EOF'
${withHeader item.content}
CARGO_TOML_EOF
    '') items}
    
    echo ""
    echo "Done! Generated ${toString (lib.length items)} Cargo.toml files."
    echo ""
    echo "Note: These files are for IDE/linting support only."
    echo "The actual build uses Nix-generated versions."
  '';

  # Helper to collect all generation targets from capabilities and modules
  collectGenerationTargets = { capabilities ? {}, modules ? {} }:
    (lib.mapAttrsToList (name: cap: {
      path = cap.srcPath or "proto/capabilities/${builtins.replaceStrings ["proto_"] [""] name}";
      content = cap.cargoTomlContent;
    }) capabilities)
    ++
    (lib.mapAttrsToList (name: mod: {
      path = mod.srcPath or "proto/modules/${builtins.replaceStrings ["proto_module"] ["module"] name}";
      content = mod.cargoTomlContent;
    }) modules);

in {
  inherit buildModule buildCapability toToml mkDep;
  inherit generatedHeader withHeader mkGenerateCargoTomlScript collectGenerationTargets;
}