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
          targets.wasm32-unknown-unknown.latest.rust-std
        ];

        craneLibNative = (crane.mkLib pkgs).overrideToolchain nativeToolchain;
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain wasmToolchain;
        craneLibNightly = (crane.mkLib pkgs).overrideToolchain nightlyToolchain;

        # Import our custom library
        myLib = import ./nix/crate.nix {
          inherit lib pkgs craneLibNative craneLibWasm;
          pyroductDep = { workspace = true; };
        };

        # Build all capabilities
        capabilities = {
          rag = (import ./capabilities/rag/capability.nix { inherit myLib; });
          cpu_client = (import ./capabilities/cpu_client/capability.nix { inherit myLib; });
          http_client = (import ./capabilities/http_client/capability.nix { inherit myLib; });
          serial_client = (import ./capabilities/serial_client/capability.nix { inherit myLib; });
        };

        # Build all modules
        modules = {
          basic = (import ./modules/basic/module.nix { inherit myLib; });
          basic_capability = (import ./modules/basic_capability/module.nix { inherit myLib; });
          rag_capability = (import ./modules/rag_capability/module.nix { inherit myLib; });
          struct_io = (import ./modules/struct_io/module.nix { inherit myLib; });
        };

        # Build the harness
        src = lib.cleanSourceWith {
          src = craneLibNative.cleanCargoSource ./.;
          filter = path: type:
            (craneLibNative.filterCargoSources path type)
            || (lib.hasSuffix ".stderr" path);
        };
        commonArgs = {
          inherit src;
          strictDeps = true;
          buildInputs = with pkgs; [ openssl ] ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.systemd ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };

        nativeArtifacts = craneLibNative.buildDepsOnly (commonArgs // {
          pname = "pyroduct-native-deps";
          version = "0.0.1";
          # Exclude the workspace members that are WASM modules
          cargoExtraArgs = "--workspace --exclude basic --exclude basic_capability --exclude rag_capability --exclude struct_io";
        });

        pyroduct = craneLibNative.buildPackage (commonArgs // {
          pname = "pyroduct";
          version = "0.1.0";
          cargoArtifacts = nativeArtifacts;
          cargoExtraArgs = "-p pyroduct";
          doCheck = false;
        });

        # Shared Library Extension
        libExt = if pkgs.stdenv.isDarwin then "dylib" else "so";

      in {
        packages = {
          inherit pyroduct;
          
          # Export capabilities (using updated names from definition)
          rag = capabilities.rag.hostPlugin;
          cpu_client = capabilities.cpu_client.hostPlugin;
          http_client = capabilities.http_client.hostPlugin;
          serial_client = capabilities.serial_client.hostPlugin;
          
          basic = modules.basic.wasm;
          basic_capability = modules.basic_capability.wasm;
          rag_capability = modules.rag_capability.wasm;
          struct_io = modules.struct_io.wasm;
        };

        lib = { inherit myLib; };

        apps.generate-cargo-toml = flake-utils.lib.mkApp {
          drv = myLib.mkGenerateCargoTomlScript (myLib.collectGenerationTargets {
            inherit capabilities modules;
          });
        };

        devShells.default = craneLibNightly.devShell {
          packages = [ nightlyToolchain pyroduct ];
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
            echo "  pyroduct                       - Run the pyroduct CLI"
            echo "  nix run .#generate-cargo-toml  - Generate Cargo.toml files"
            echo "  nix run .#run-tests            - Run the test harness"
          '';
        };
      }
    );
}