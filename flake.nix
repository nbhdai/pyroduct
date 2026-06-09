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
    process-compose-flake = {
      url = "github:Platonic-Systems/process-compose-flake";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      crane,
      process-compose-flake,
      ...
    }:
    {
      nixosModules.pyro-daemon = import ./nix/pyro-daemon-module.nix;
      nixosModules.default = import ./nix/pyro-daemon-module.nix;
    }
    //
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;

        # Toolchains
        nativeToolchain = fenix.packages.${system}.stable.toolchain;
        wasmToolchain =
          with fenix.packages.${system};
          combine [
            stable.toolchain
            targets.wasm32-unknown-unknown.stable.rust-std
            stable.rust-src
            stable.rust-analyzer
          ];

        miriToolchain =
          with fenix.packages.${system};
          combine [
            complete.toolchain
          ];

        craneLibNative = (crane.mkLib pkgs).overrideToolchain nativeToolchain;
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain wasmToolchain;
        craneLibMiri = (crane.mkLib pkgs).overrideToolchain miriToolchain;

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
          src = ./lib;
          filter =
            path: type:
            (craneLibNative.filterCargoSources path type)
            || (lib.hasSuffix "tauri.conf.json" path)
            || (lib.hasSuffix ".png" path);
        };
        commonPyroArgs = {
          src = pyroSrc;
          strictDeps = true;
          buildInputs = with pkgs; [ openssl ] ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.systemd
            pkgs.glib
            pkgs.gtk3
            pkgs.webkitgtk_4_1
            pkgs.libsoup_3
            pkgs.librsvg
            pkgs.bzip2
          ];
          nativeBuildInputs = with pkgs; [ pkg-config ];
        };

        pyroductDeps = craneLibNative.buildDepsOnly (
          commonPyroArgs
          // {
            pname = "pyroduct-deps";
          }
        );

        pyroduct = craneLibNative.buildPackage (
          commonPyroArgs
          // {
            pname = "pyroduct";
            version = "0.1.0";
            cargoArtifacts = pyroductDeps;
            doCheck = false;
            cargoExtraArgs = "-p pyroduct --features cli";
          }
        );

        pyro-daemon = craneLibNative.buildPackage (
          commonPyroArgs
          // {
            pname = "pyro-daemon";
            version = "0.1.0";
            cargoArtifacts = pyroductDeps;
            doCheck = false;
            cargoExtraArgs = "-p pyro-daemon";
          }
        );

        installTests = import ./nix/install-tests.nix { inherit pkgs pyroduct; };
        moduleTests = import ./nix/module-tests.nix { inherit pkgs pyroduct pyro-daemon; };
        miriTests = import ./nix/miri-tests.nix {
          inherit pkgs miriToolchain;
          craneLib = craneLibMiri;
          commonArgs = commonPyroArgs;
        };
        rustTests = import ./nix/rust-tests.nix {
          inherit pkgs;
          craneLib = craneLibNative;
          commonArgs = commonPyroArgs;
          pyroduct = pyroduct;
        };

        devGui = pkgs.writeShellScriptBin "dev-gui" ''
          cd lib/pyro-gui
          exec ${pkgs.bacon}/bin/bacon --job gui "$@"
        '';

        guiDev = import ./nix/gui-dev.nix { inherit pkgs process-compose-flake; };

      in
      {
        lib = {
          makeProcessCompose = args: import ./nix/gui-dev.nix ({
            inherit pkgs process-compose-flake;
          } // args);

          makeGuiDev = args: import ./nix/gui-dev.nix ({
            inherit pkgs process-compose-flake;
          } // args);

          makeGuiBuild = args: import ./nix/gui-build.nix ({
            inherit pkgs process-compose-flake pyro-daemon;
          } // args);
        };

        packages = {
          inherit pyroduct;
          pyro-daemon = pyro-daemon;
          dev-gui = devGui;
          process-compose = guiDev;
          default = pyroduct;
        };

        checks = {
          nixos-module-test = moduleTests.check;
          install-script-test = installTests.install-script-check;
          miri-tests = miriTests.check;
          rust-tests = rustTests.check;
        };

        apps = {
          default = flake-utils.lib.mkApp { drv = pyroduct; };
          dev-gui = flake-utils.lib.mkApp { drv = devGui; };
          process-compose = flake-utils.lib.mkApp { drv = guiDev; };
        }
        // (lib.optionalAttrs pkgs.stdenv.isLinux {
          valgrind-test = {
            type = "app";
            program = toString (
              pkgs.writeShellScript "valgrind-test" ''
                set -e

                TEST_NAME="''${1:?Usage: nix run .#valgrind-test <test-name> [-- <valgrind-args>]}"
                shift

                echo "Building test binary for: $TEST_NAME"
                cargo test --manifest-path lib/pyroduct/Cargo.toml --all-features --no-run --test "$TEST_NAME" 2>&1 

                # Find the compiled test binary
                BIN=$(cargo test --manifest-path lib/pyroduct/Cargo.toml --all-features --no-run --test "$TEST_NAME" --message-format=json 2>/dev/null \
                  | ${pkgs.jq}/bin/jq -r 'select(.executable != null) | .executable' \
                  | tail -1)

                if [ -z "$BIN" ]; then
                  echo "Error: could not find test binary for '$TEST_NAME'"
                  exit 1
                fi

                echo "Running: valgrind $@ $BIN"
                exec ${pkgs.valgrind}/bin/valgrind "$@" "$BIN"
              ''
            );
          };
        });

        devShells.default = craneLibWasm.devShell (
          wasmEnv
          // {
            buildInputs = commonPyroArgs.buildInputs;
            nativeBuildInputs = commonPyroArgs.nativeBuildInputs;
            packages = [
              wasmToolchain
              pyroduct
              installTests.bin
              miriTests.bin
              rustTests.bin
              rustTests.prepare
              devGui
              guiDev
              pkgs.jq
              pkgs.bzip2
              pkgs.cargo-expand
              pkgs.nodejs
              pkgs.cargo-tauri
              pkgs.bacon
              pkgs.pkg-config
            ]
            ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.valgrind ];
            RUST_SRC_PATH = "${wasmToolchain}/lib/rustlib/src/rust/library";
            RUST_ANALYZER_PATH = "${wasmToolchain}/bin/rust-analyzer";
            CARGO = "${wasmToolchain}/bin/cargo";
            RUSTUP_TOOLCHAIN = "${wasmToolchain}";

            shellHook = ''
              export PYRODUCT="$PWD/test"
              export PYRO_DAEMON_DIR="$PWD/test/"
              export PYRODUCT_ROOT="$PWD/test/"

              ${lib.optionalString pkgs.stdenv.isLinux ''
                export LD_LIBRARY_PATH="${lib.makeLibraryPath commonPyroArgs.buildInputs}:''${LD_LIBRARY_PATH:-}"
              ''}
              ${lib.optionalString pkgs.stdenv.isDarwin ''
                unset DEVELOPER_DIR
                echo "  - SDKROOT: ''${SDKROOT:-default}"
              ''}

              echo "Development shell loaded!"
              echo ""
              echo "Available commands:"
              echo "  pyroduct prepare-pyro test-rust dev-gui process-compose"
              echo ""
            '';
          }
        );
      }
    );
}
