{
  description = "Pyroduct Experiments Harness and Plugins";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
    };
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;

        # Toolchains
        nativeToolchain = fenix.packages.${system}.stable.minimalToolchain;
        wasmToolchain = with fenix.packages.${system}; combine [
          stable.minimalToolchain
          targets.wasm32-unknown-unknown.stable.rust-std
        ];
        nightlyToolchain = with fenix.packages.${system}; combine [
          latest.toolchain
          # targets.x86_64-unknown-linux-gnu.latest.rust-std
          targets.wasm32-unknown-unknown.latest.rust-std
        ];

        craneLibNative = (crane.mkLib pkgs).overrideToolchain nativeToolchain;
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain wasmToolchain;
        craneLibNightly = (crane.mkLib pkgs).overrideToolchain nightlyToolchain;

        # Import our custom library
        myLib = import ./lib/crate.nix {
          inherit lib pkgs craneLibNative craneLibWasm;
          workspaceRoot = ./.;
          pyroductPath = "../../../lib/pyroduct";
        };

        # Build all capabilities first
        capabilities = {
          proto_reporter = (import ./proto/capabilities/reporter/capability.nix { inherit myLib; });
          proto_cpu_info = (import ./proto/capabilities/cpu_info/capability.nix { inherit myLib; });
          proto_http_client = (import ./proto/capabilities/http_client/capability.nix { inherit myLib; });
          proto_serial_client = (import ./proto/capabilities/serial_client/capability.nix { inherit myLib; });
        };

        # Build all modules
        modules = {
          proto_module = (import ./proto/modules/module/module.nix { inherit myLib capabilities; });
          proto_module_2 = (import ./proto/modules/module_2/module.nix { inherit myLib capabilities; });
          proto_module_3 = (import ./proto/modules/module_3/module.nix { inherit myLib capabilities; });
        };

        # Build the harness (still using traditional approach for now)
        src = craneLibNative.cleanCargoSource ./.;
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = with pkgs; [ openssl ] ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.systemd ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };

        nativeArtifacts = craneLibNative.buildDepsOnly (commonArgs // {
          pname = "pyroduct-native-deps";
          version = "0.0.1";
          cargoExtraArgs = "--workspace --exclude proto_module --exclude proto_module_2 --exclude proto_module_3";
        });

        harness = craneLibNative.buildPackage (commonArgs // {
          pname = "harness";
          version = "0.1.0";
          cargoArtifacts = nativeArtifacts;
          cargoExtraArgs = "-p harness";
        });

        # Shared Library Extension
        libExt = if pkgs.stdenv.isDarwin then "dylib" else "so";

        # Generate config files using the new module/capability structure
        mkConfig = { moduleDef, extraInputs ? [] }: pkgs.writeText "${moduleDef.name}-config.json" (builtins.toJSON {
          module_name = moduleDef.name;
          module = "${moduleDef.wasm}/lib/${moduleDef.name}.wasm";
          capabilities = map (cap: "${cap.hostPlugin}/lib/lib${cap.name}.${libExt}") moduleDef.capabilities;
          input = extraInputs;
        });

        config1 = mkConfig {
          moduleDef = modules.proto_module;
          extraInputs = [
            { input = "Hello World from Host"; }
            { input = "This is a second input"; }
          ];
        };

        config2 = mkConfig {
          moduleDef = modules.proto_module_2;
          extraInputs = [
            { input = "https://httpbin.org/get"; }
            { input = "this should fail"; }
          ];
        };

        config3 = mkConfig {
          moduleDef = modules.proto_module_3;
          extraInputs = [
            {
              input = {
                port = "/dev/ttyUSB0";
                baud = 9600;
                command = "AT";
              };
            }
          ];
        };

      in {
        packages = {
          inherit harness;
          
          # Export individual capabilities
          proto_reporter = capabilities.proto_reporter.hostPlugin;
          proto_cpu_info = capabilities.proto_cpu_info.hostPlugin;
          proto_http_client = capabilities.proto_http_client.hostPlugin;
          proto_serial_client = capabilities.proto_serial_client.hostPlugin;
          
          # Export individual modules
          proto_module = modules.proto_module.wasm;
          proto_module_2 = modules.proto_module_2.wasm;
          proto_module_3 = modules.proto_module_3.wasm;
          
          default = harness;
        };

        # Expose the library for external use
        lib = { inherit myLib; };

        apps.run-tests = flake-utils.lib.mkApp {
          drv = pkgs.writeShellScriptBin "run-tests" ''
            echo "==================================="
            echo "Testing Module 1 (Reporter)"
            echo "Config: ${config1}"
            ${harness}/bin/harness ${config1}
            echo ""

            echo "==================================="
            echo "Testing Module 2 (CPU + HTTP)"
            echo "Config: ${config2}"
            ${harness}/bin/harness ${config2}
            echo ""
            
            echo "==================================="
            echo "Testing Module 3 (Serial)"
            echo "Config: ${config3}"
            ${harness}/bin/harness ${config3}
            echo ""
          '';
        };

        # Debug app to show generated Cargo.toml files
        apps.show-cargo-toml = flake-utils.lib.mkApp {
          drv = pkgs.writeShellScriptBin "show-cargo-toml" ''
            echo "=== proto_reporter Cargo.toml ==="
            echo "${capabilities.proto_reporter.cargoTomlContent}"
            echo ""
            echo "=== proto_module Cargo.toml ==="
            echo "${modules.proto_module.cargoTomlContent}"
            echo ""
          '';
        };

        # Generate Cargo.toml files for IDE/linting support
        apps.generate-cargo-toml = flake-utils.lib.mkApp {
          drv = myLib.mkGenerateCargoTomlScript (myLib.collectGenerationTargets {
            inherit capabilities modules;
          });
        };

        devShells.default = craneLibNightly.devShell {
          packages = [ nightlyToolchain ];
          
          RUST_SRC_PATH = "${nightlyToolchain}/lib/rustlib/src/rust/library";
          CARGO = "${nightlyToolchain}/bin/cargo";
          RUSTUP_TOOLCHAIN = "${nightlyToolchain}";
          
          shellHook = ''
            ${lib.optionalString pkgs.stdenv.isLinux ''
              export LD_LIBRARY_PATH="${lib.makeLibraryPath [ pkgs.systemd ]}:''${LD_LIBRARY_PATH:-}"
            ''}
            ${lib.optionalString pkgs.stdenv.isDarwin ''
              unset DEVELOPER_DIR
            ''}
            
            echo "Development shell loaded!"
            echo ""
            echo "Available commands:"
            echo "  nix run .#generate-cargo-toml  - Generate Cargo.toml files for IDE support"
            echo "  nix run .#show-cargo-toml      - Preview generated Cargo.toml content"
            echo "  nix run .#run-tests            - Run the test harness"
          '';
        };
      }
    );
}