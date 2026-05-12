{ pkgs, craneLibNative, craneLibWasm, pyroductTool, pyroductSrc }:

let
  # Step 1: Create minimal manifest and source
  mkMinimalSrc = { type, name, version, code, capDeps ? [] }:
    pkgs.runCommand "minimal-src-${type}-${name}-${version}" {
      nativeBuildInputs = [ pkgs.bash ];
    } ''
      mkdir -p $out/src
      echo "${code}" > $out/src/lib.rs
      
      if [ "${type}" = "module" ]; then
        echo "[module]
author = \"nbhdai\"
name = \"${name}\"
version = \"${version}\"

[pyroduct]
path = \"${pyroductSrc}/pyroduct\"

[capabilities]
${builtins.concatStringsSep "\n" (map (cap: "${cap.name} = { package = \"${cap.name}\", author = \"nbhdai\", version = \"${cap.version}\" }") capDeps)}

[dependencies]" > $out/Module.toml
      else # capability or interface
        echo "[capability]
author = \"nbhdai\"
name = \"${name}\"
version = \"${version}\"

[pyroduct]
path = \"${pyroductSrc}/pyroduct\"

[dependencies.host]
[dependencies.module]
[dependencies.shared]" > $out/Capability.toml
      fi
    '';

  # Step 2: Run pyroduct expand to generate Cargo.toml and FFI glue
  expandProject = { type, name, version, code ? null, src, capDeps ? [] }:
    let
      projectSrc = if src != null then src else mkMinimalSrc { 
        type = type; 
        name = name; 
        version = version; 
        code = code; 
        capDeps = capDeps; 
      };
    in

    pkgs.runCommand "expanded-src-${type}-${name}-${version}" {
      nativeBuildInputs = [ pyroductTool pkgs.cargo pkgs.rustc ];
    } ''
      cp -a ${projectSrc}/. .
      chmod -R u+w .
      ls -R
      
      # Fix relative paths to pyroduct in manifest files
      if [ -f Capability.toml ]; then
        sed -i "s|path = \".*\"|path = \"${pyroductSrc}/pyroduct\"|" Capability.toml
      fi
      if [ -f Module.toml ]; then
        sed -i "s|path = \".*\"|path = \"${pyroductSrc}/pyroduct\"|" Module.toml
      fi

      # pyroduct expand needs a cache dir if dependencies are listed
      export PYRODUCT=$out/cache
      mkdir -p $out/cache
      pyroduct expand --no-compile .
      cargo generate-lockfile
    '';

in
{
  interfaceBuild = { name, version, code ? null, src ? null }:
    let
      src_expanded = expandProject { 
        type = "interface"; 
        name = name; 
        version = version; 
        code = code; 
        src = src;
      };
    in

    pkgs.stdenv.mkDerivation {
      pname = "interface-${name}";
      version = version;
      
      nativeBuildInputs = [ pyroductTool ];
      
      buildPhase = ''
        mkdir -p build
        cp -r ${src_expanded} build/
        cd build
        export PYRODUCT=$out
        pyroduct package .
      '';
      
      installPhase = ''
        mkdir -p $out/interfaces/${name}/${version}
        if [ -d build/artifacts ]; then
          cp -r build/artifacts/* $out/interfaces/${name}/${version}/
        else
          cp -r build/* $out/interfaces/${name}/${version}/
        fi
        find $out/interfaces/${name}/${version} -name "target" -exec rm -rf {} +
      '';
    };

  capabilityBuild = { name, version, code ? null, src ? null, interfaces ? [], capabilities ? [] }:
    let
      src_expanded = expandProject { 
        type = "capability"; 
        name = name; 
        version = version; 
        code = code; 
        src = src;
      };
      
      libDerivation = craneLibNative.buildPackage {
        pname = "lib-${name}";
        version = version;
        src = src_expanded;
        cargoExtraArgs = "--lib";
        cargoVendorDir = null;
      };
    in

    pkgs.stdenv.mkDerivation {
      pname = "capability-${name}";
      version = version;
      
      nativeBuildInputs = [ pyroductTool ];
      
      buildPhase = ''
        mkdir -p build
        cp -r ${src_expanded} build/
        cd build
        export PYRODUCT=$out
        
        # Populate cache with dependencies
        ${builtins.concatStringsSep "\n" (map (dep: "mkdir -p $out/interfaces/${dep.name}/${dep.version} && cp -r ${dep}/interfaces/${dep.name}/${dep.version}/* $out/interfaces/${dep.name}/${dep.version}/") interfaces)}
        ${builtins.concatStringsSep "\n" (map (dep: "mkdir -p $out/capabilities/${dep.name}/${dep.version} && cp -r ${dep}/capabilities/${dep.name}/${dep.version}/* $out/capabilities/${dep.name}/${dep.version}/") capabilities)}
        
        pyroduct package .
      '';
      
      installPhase = ''
        mkdir -p $out/capabilities/${name}/${version}
        if [ -d build/artifacts ]; then
          cp -r build/artifacts/* $out/capabilities/${name}/${version}/
        else
          cp -r build/* $out/capabilities/${name}/${version}/
        fi
        
        LIB_FILE=$(find ${libDerivation} -name "*.dylib" -o -name "*.so" | head -n 1)
        if [ -n "$LIB_FILE" ]; then
          cp "$LIB_FILE" $out/capabilities/${name}/${version}/lib.dylib
        fi
        
        find $out/capabilities/${name}/${version} -name "target" -exec rm -rf {} +
      '';
    };

  moduleBuild = { name, interfaces, capabilities, code ? null, src ? null }:
    let
      capDeps = map (c: { name = c.name; version = c.version; }) capabilities;
      src_expanded = expandProject { 
        type = "module"; 
        name = name; 
        version = "0.1.0"; 
        code = code; 
        capDeps = capDeps; 
        src = src;
      };
      
      wasmDerivation = craneLibWasm.buildPackage {
        pname = "mod-${name}";
        version = "0.1.0";
        src = src_expanded;
        cargoVendorDir = null;
      };
    in

    pkgs.stdenv.mkDerivation {
      pname = "module-${name}";
      version = "0.1.0";
      
      nativeBuildInputs = [ pyroductTool ];
      
      buildPhase = ''
        mkdir -p build
        cp -r ${src_expanded} build/
        cd build
        export PYRODUCT=$out
        
        ${builtins.concatStringsSep "\n" (map (dep: "mkdir -p $out/interfaces/${dep.name}/${dep.version} && cp -r ${dep}/interfaces/${dep.name}/${dep.version}/* $out/interfaces/${dep.name}/${dep.version}/") interfaces)}
        ${builtins.concatStringsSep "\n" (map (dep: "mkdir -p $out/capabilities/${dep.name}/${dep.version} && cp -r ${dep}/capabilities/${dep.name}/${dep.version}/* $out/capabilities/${dep.name}/${dep.version}/") capabilities)}
        
        pyroduct package .
      '';
      
      installPhase = ''
        mkdir -p $out/artifacts
        if [ -d build/artifacts ]; then
          cp -r build/artifacts/* $out/artifacts/
        else
          cp -r build/* $out/artifacts/
        fi
        
        WASM_FILE=$(find ${wasmDerivation} -name "*.wasm" | head -n 1)
        if [ -n "$WASM_FILE" ]; then
          cp "$WASM_FILE" $out/artifacts/mod.wasm
        fi
        
        find $out/artifacts -name "target" -exec rm -rf {} +
      '';
    };
}
