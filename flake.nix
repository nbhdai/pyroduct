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
            echo "  pyroduct"
            echo ""
          '';
        };
      }
    );
}