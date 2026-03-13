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

        wasmEnv = {
          nativeBuildInputs = [
            pkgs.llvmPackages.clang-unwrapped
            pkgs.llvmPackages.lld
          ];
          CC_wasm32_unknown_unknown = "${pkgs.llvmPackages.clang-unwrapped}/bin/clang";
          CXX_wasm32_unknown_unknown = "${pkgs.llvmPackages.clang-unwrapped}/bin/clang++";
          LD_wasm32_unknown_unknown = "${pkgs.llvmPackages.lld}/bin/lld";
        };

        # Build the harness
        pyroSrc = lib.cleanSourceWith {
          src = craneLibNative.cleanCargoSource ./lib;
          filter = path: type:
            (craneLibNative.filterCargoSources path type)
            || (lib.hasSuffix ".stderr" path);
        };
        commonPyroArgs = {
          src = pyroSrc;
          strictDeps = true;
          buildInputs = with pkgs; [ openssl ] ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.systemd ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };

        pyroductDeps = craneLibNative.buildDepsOnly (commonPyroArgs // {
          pname = "pyroduct-deps";
        });

        pyroduct = craneLibNative.buildPackage (commonPyroArgs // {
          pname = "pyroduct";
          version = "0.1.0";
          cargoArtifacts = pyroductDeps;
          doCheck = false;
          cargoExtraArgs = "-p pyroduct-cli";
          postInstall = ''
            mv $out/bin/pyroduct-cli $out/bin/pyroduct
          '';
        });

        # Shared Library Extension
        libExt = if pkgs.stdenv.isDarwin then "dylib" else "so";

      in {
        packages = {
          inherit pyroduct;
        };

        apps.valgrind-test = lib.optionals pkgs.stdenv.isLinux {
          type = "app";
          program = toString (pkgs.writeShellScript "valgrind-test" ''
            set -e

            TEST_NAME="''${1:?Usage: nix run .#valgrind-test <test-name> [-- <valgrind-args>]}"
            shift

            echo "Building test binary for: $TEST_NAME"
            cargo test --no-run --test "$TEST_NAME" 2>&1

            # Find the compiled test binary
            BIN=$(cargo test --no-run --test "$TEST_NAME" --message-format=json 2>/dev/null \
              | ${pkgs.jq}/bin/jq -r 'select(.executable != null) | .executable' \
              | tail -1)

            if [ -z "$BIN" ]; then
              echo "Error: could not find test binary for '$TEST_NAME'"
              exit 1
            fi

            echo "Running: valgrind $@ $BIN"
            exec ${pkgs.valgrind}/bin/valgrind "$@" "$BIN"
          '');
        };

        devShells.default = craneLibNightly.devShell (wasmEnv // {
          packages = [ 
            nightlyToolchain
            pyroduct
          ] ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.valgrind ];
          RUST_SRC_PATH = "${nightlyToolchain}/lib/rustlib/src/rust/library";
          CARGO = "${nightlyToolchain}/bin/cargo";
          RUSTUP_TOOLCHAIN = "${nightlyToolchain}";
          
          shellHook = ''
            ${lib.optionalString pkgs.stdenv.isLinux ''
              export LD_LIBRARY_PATH="${lib.makeLibraryPath [ pkgs.systemd ]}:''${LD_LIBRARY_PATH:-}"
            ''}
            ${lib.optionalString pkgs.stdenv.isDarwin ''
              unset DEVELOPER_DIR
              echo "  - SDKROOT: ''${SDKROOT:-default}"
            ''}`
            
            echo "Development shell loaded!"
            echo ""
            echo "Available commands:"
            echo "  pyroduct"
            echo ""
          '';
        });
      }
    );
}