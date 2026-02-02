{ pkgs, craneLibNative, craneLibWasm, pyroduct }:

let
  tomlFormat = pkgs.formats.toml {};
  lib = pkgs.lib;

  # Enhanced Helper: Now accepts an optional subDir (e.g., "interface")
  processDeps = subDir: deps: 
    if deps == null then { copyCmds = ""; patched = null; }
    else
      let
        pathDeps = lib.filterAttrs (n: v: builtins.isAttrs v && v ? path) deps;
        
        copyCmds = lib.concatStringsSep "\n" (lib.mapAttrsToList (name: meta: ''
          mkdir -p $out/deps/${name}
          cp -r ${meta.path}/* $out/deps/${name}/
        '') pathDeps);

        patched = lib.mapAttrs (name: v: 
          if builtins.isAttrs v && v ? path then 
            let 
              # Mirror the CLI logic: if it's a capability, point to the interface subdir
              finalPath = if subDir != null then "./deps/${name}/${subDir}" else "./deps/${name}";
            in v // { path = finalPath; } 
          else v
        ) deps;
      in { inherit copyCmds patched; };

  buildPyroductProject = {
    name,
    src,
    type,
    pyroductLibSrc,
    config ? null,   
    configFile ? null,
    lockFile ? null,
    cargoArtifacts ? null,
    ...
  }@args:
  let
    loadedConfig = if configFile != null then import configFile else config;

    # Revised Patching Logic
    processedConfig = if loadedConfig == null then null else
      let
        # Helper to apply the subDir logic to a specific path in the attrset
        patchSection = subDir: sectionPath: attrs:
          let
            currentDeps = lib.attrByPath sectionPath {} attrs;
            processed = processDeps subDir currentDeps;
          in {
            inherit (processed) copyCmds;
            newConfig = lib.setAttrByPath sectionPath processed.patched attrs;
          };

        mergeResults = results: {
          copyCmds = lib.concatMapStrings (r: r.copyCmds) results;
          newConfig = lib.foldl' (cfg: r: lib.recursiveUpdate cfg r.newConfig) loadedConfig results;
        };

      in if type == "module" then
        # For modules, we must process standard [dependencies] AND [capabilities]
        # [capabilities] paths are redirected to the "interface" folder 
        mergeResults [
          (patchSection null ["dependencies"] loadedConfig)
          (patchSection "interface" ["capabilities"] loadedConfig)
        ]
      else
        # For capabilities, process the three distinct dependency groups 
        mergeResults [
          (patchSection null ["dependencies" "host"] loadedConfig)
          (patchSection null ["dependencies" "module"] loadedConfig)
          (patchSection null ["dependencies" "shared"] loadedConfig)
        ];

    finalConfig = if processedConfig != null then processedConfig.newConfig else null;
    depCopyCmds = if processedConfig != null then processedConfig.copyCmds else "";

    nixConfig = if finalConfig != null then 
      let
        existingPyro = finalConfig.pyroduct or {};
        patchedPyro = existingPyro // { path = "./pyroduct_lib"; };
      in
        finalConfig // { pyroduct = patchedPyro; }
    else null;

    tomlFilename = if type == "module" then "Module.toml" else "Capability.toml";
    tomlFile = if nixConfig != null then 
      tomlFormat.generate "${name}-${tomlFilename}" nixConfig
    else null;

    preparedSource = pkgs.runCommand "${name}-src" {
      nativeBuildInputs = [ pyroduct ];
    } ''
      mkdir -p $out
      cp -r ${src}/* $out/
      chmod -R +w $out

      ${pkgs.lib.optionalString (lockFile != null) ''
        cp ${lockFile} $out/Cargo.lock
      ''}

      ${pkgs.lib.optionalString (tomlFile != null) ''
        rm -f $out/${tomlFilename}
        cp ${tomlFile} $out/${tomlFilename}
      ''}

      mkdir -p $out/pyroduct_lib
      cp -r ${pyroductLibSrc}/* $out/pyroduct_lib/

      ${depCopyCmds}
      
      cd $out
      pyroduct expand .
    '';

    craneLib = if type == "module" then craneLibWasm else craneLibNative;
    baseArgs = removeAttrs args [ "config" "configFile" "lockFile" "pyroductLibSrc" "type" ];
    vendorArgs = if lockFile != null then {
      cargoVendorDir = craneLib.vendorCargoDeps { inherit lockFile; };
    } else {};

    commonCargoArgs = baseArgs // vendorArgs // {
      src = preparedSource;
      pname = name;
      cargoExtraArgs = if type == "module" 
        then "--target wasm32-unknown-unknown ${args.cargoExtraArgs or ""}"
        else "--features capability ${args.cargoExtraArgs or ""}"; [cite: 1, 5]
    };

    artifacts = if cargoArtifacts != null then cargoArtifacts 
      else craneLib.buildDepsOnly (commonCargoArgs // {
        pname = "${name}-deps";
      });

  in craneLib.buildPackage (commonCargoArgs // {
    cargoArtifacts = artifacts;
    
    installPhase = ''
      mkdir -p $out/bin $out/lib $out/artifacts $out/interface
      cp Cargo.toml $out/Cargo.toml

      if [ "${type}" = "module" ]; then
        cp target/wasm32-unknown-unknown/release/*.wasm $out/bin/${name}.wasm
        if [ -d artifacts ]; then cp -r artifacts/* $out/artifacts/; fi 
      else
        find target/release -maxdepth 1 -type f \( -name "*.so" -o -name "*.dylib" -o -name "*.dll" \) -exec cp {} $out/lib/ \;
        if [ -d interface ]; then cp -r interface/* $out/interface/; fi [cite: 3, 5]
      fi
    '';
  });

in {
  buildModule = args: buildPyroductProject (args // { type = "module"; });
  buildCapability = args: buildPyroductProject (args // { type = "capability"; });
}