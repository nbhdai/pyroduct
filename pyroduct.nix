{
  pkgs,
  craneLibNative,
  craneLibWasm,
  pyroductTool,
  pyroductSrc,
}:

let
  # Run pyroduct expand to generate Cargo.toml and FFI glue
  expandProject =
    {
      type,
      name,
      version,
      src,
      interfaces ? [ ],
      capabilities ? [ ],
    }:
    pkgs.runCommand "expanded-src-${type}-${name}-${version}"
      {
        nativeBuildInputs = [
          pyroductTool
          pkgs.cargo
          pkgs.rustc
        ];
      }
      ''
        mkdir -p $out
        cp -a ${src}/. $out/
        chmod -R u+w $out/
        cd $out

        # Fix relative paths to pyroduct in manifest files
        mkdir -p $out/pyroduct-src
        cp -r ${pyroductSrc}/* $out/pyroduct-src/
        for f in Capability.toml Module.toml Cargo.toml; do
          if [ -f "$f" ]; then
            sed -i 's|path = "[^"]*lib/pyroduct"|path = "./pyroduct-src/pyroduct"|g' "$f"
          fi
        done

        # pyroduct expand needs a cache dir if dependencies are listed
        export PYRODUCT=./cache
        mkdir -p ./cache

        # Populate cache with dependencies before expansion
        ${builtins.concatStringsSep "\n" (
          map (
            dep:
            "mkdir -p ./cache/interfaces/${dep.author}/${dep.name}/${dep.version} && cp -r ${dep.drv}/${dep.author}/${dep.name}/${dep.version}/* ./cache/interfaces/${dep.author}/${dep.name}/${dep.version}/ || true"
          ) interfaces
        )}
        ${builtins.concatStringsSep "\n" (
          map (
            dep:
            "mkdir -p ./cache/capabilities/${dep.author}/${dep.name}/${dep.version} && cp -r ${dep.drv}/${dep.author}/${dep.name}/${dep.version}/* ./cache/capabilities/${dep.author}/${dep.name}/${dep.version}/ || true"
          ) capabilities
        )}

        pyroduct expand --no-compile .
      '';

in
{
  interfaceBuild =
    {
      name,
      version,
      src,
      author,
    }:
    let
      src_expanded = expandProject {
        type = "interface";
        name = name;
        version = version;
        src = src;
      };
      drv = pkgs.stdenv.mkDerivation {
        pname = "interface-${name}";
        version = version;
        src = src_expanded;

        nativeBuildInputs = [
          pyroductTool
          pkgs.cargo
          pkgs.rustc
          pkgs.pkg-config
          pkgs.openssl
          pkgs.cacert
        ];

        dontBuild = true;

        installPhase = ''
          mkdir -p $out/${author}/${name}/${version}
          pyroduct ship . --out $out/${author}/${name}/${version}
          find $out/${author}/${name}/${version} -name "target" -exec rm -rf {} + || true
        '';
      };
    in
    {
      inherit drv;
      name = name;
      version = version;
      author = author;
    };

  capabilityBuild =
    {
      name,
      version,
      src,
      interfaces ? [ ],
      capabilities ? [ ],
      author,
    }:
    let
      src_expanded = expandProject {
        type = "capability";
        name = name;
        version = version;
        src = src;
        interfaces = interfaces;
        capabilities = capabilities;
      };

      libDerivation = craneLibNative.buildPackage {
        pname = "lib-${name}";
        version = version;
        src = src_expanded;
        preBuild = "cp -r $src/. .";
        cargoExtraArgs = "--lib";
      };

      drv = pkgs.stdenv.mkDerivation {
        pname = "capability-${name}";
        version = version;
        src = src_expanded;

        nativeBuildInputs = [
          pyroductTool
          pkgs.cargo
          pkgs.rustc
          pkgs.pkg-config
          pkgs.openssl
          pkgs.cacert
        ];

        installPhase = ''
          mkdir -p $out/${author}/${name}/${version}

          LIB_FILE=$(find ${libDerivation} -name "*.dylib" -o -name "*.so" -o -name "*.dll" | head -n 1)
          if [ -n "$LIB_FILE" ]; then
            EXT="''${LIB_FILE##*.}"
            cp "$LIB_FILE" $out/${author}/${name}/${version}/lib.$EXT
          fi

          pyroduct spec . --out $out/${author}/${name}/${version}/interface.json
        '';
      };
    in
    {
      inherit drv;
      name = name;
      version = version;
      author = author;
    };

  moduleBuild =
    {
      name,
      interfaces,
      capabilities,
      src,
      author,
    }:
    let
      capDeps = map (c: {
        name = c.name;
        version = c.version;
      }) capabilities;
      src_expanded = expandProject {
        type = "module";
        name = name;
        version = "0.1.0";
        src = src;
        interfaces = interfaces;
        capabilities = capabilities;
      };

      wasmDerivation = craneLibWasm.buildPackage {
        pname = "mod-${name}";
        version = "0.1.0";
        src = src_expanded;
        preBuild = "cp -r $src/. .";
      };

      drv = pkgs.stdenv.mkDerivation {
        pname = "module-${name}";
        version = "0.1.0";
        src = src_expanded;

        nativeBuildInputs = [
          pyroductTool
          pkgs.cargo
          pkgs.rustc
          pkgs.pkg-config
          pkgs.openssl
          pkgs.cacert
        ];

        buildPhase = ''
          export PYRODUCT=./cache
          mkdir -p ./cache

          ${builtins.concatStringsSep "\n" (
            map (
              dep:
              "mkdir -p ./cache/interfaces/${dep.author}/${dep.name}/${dep.version} && cp -r ${dep.drv}/${dep.author}/${dep.name}/${dep.version}/* ./cache/interfaces/${dep.author}/${dep.name}/${dep.version}/ || true"
            ) interfaces
          )}
          ${builtins.concatStringsSep "\n" (
            map (
              dep:
              "mkdir -p ./cache/capabilities/${dep.author}/${dep.name}/${dep.version} && cp -r ${dep.drv}/${dep.author}/${dep.name}/${dep.version}/* ./cache/capabilities/${dep.author}/${dep.name}/${dep.version}/ || true"
            ) capabilities
          )}

          mkdir -p ./artifacts
          pyroduct ship . --out ./artifacts/
        '';

        installPhase = ''
          mkdir -p $out/artifacts
          if [ -d ./artifacts ]; then
            cp -r ./artifacts/* $out/artifacts/ || true
          fi

          find $out/artifacts -name "target" -exec rm -rf {} + || true

          WASM_FILE=$(find ${wasmDerivation} -name "*.wasm" | head -n 1)
          if [ -n "$WASM_FILE" ]; then
            cp "$WASM_FILE" $out/artifacts/mod.wasm
          fi
        '';
      };
    in
    {
      inherit drv;
      name = name;
      version = "0.1.0";
      author = author;
    };
}
