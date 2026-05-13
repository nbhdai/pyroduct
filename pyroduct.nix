{
  pkgs,
  craneLibNative,
  craneLibWasm,
  pyroductTool,
  pyroductSrc,
}:

{
  interfaceBuild =
    {
      name,
      version,
      src,
      author,
    }:
    let
      drv = pkgs.stdenv.mkDerivation {
        pname = "interface-${name}";
        version = version;
        src = src;

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
          pyroduct ship . --out ./artifacts/
        '';

        installPhase = ''
          mkdir -p $out/${author}/${name}/${version}
          cp -r ./artifacts/* $out/${author}/${name}/${version}/
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
      author,
    }:
    let
      libDerivation = craneLibNative.buildPackage {
        pname = "lib-${name}";
        version = version;
        src = src;
        cargoVendorDir = null;
        cargoExtraArgs = "--lib";
      };

      drv = pkgs.stdenv.mkDerivation {
        pname = "capability-${name}";
        version = version;
        src = src;

        nativeBuildInputs = [
          pyroductTool
          pkgs.cargo
          pkgs.rustc
          pkgs.pkg-config
          pkgs.openssl
          pkgs.cacert
        ];

        installPhase = ''
          export PYRODUCT=$TMPDIR
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

      drv = pkgs.stdenv.mkDerivation {
        pname = "module-${name}";
        version = "0.1.0";
        src = src;

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
          mkdir -p $out/
          if [ -d ./artifacts ]; then
            cp -r ./artifacts/* $out/ || true
          fi

          find $out/artifacts -name "target" -exec rm -rf {} + || true

          WASM_FILE=$(find ./artifacts -name "*.wasm" | head -n 1)
          if [ -n "$WASM_FILE" ]; then
            cp "$WASM_FILE" $out/mod.wasm
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
