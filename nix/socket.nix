{ pkgs, craneLibNative, craneLibWasm, pyroduct, pyroductSrc }:

let
  # Use the new build system to create a real set of components for the test
  pyro = import ../pyroduct.nix { 
    inherit pkgs craneLibNative craneLibWasm; 
    pyroductTool = pyroduct; 
    pyroductSrc = pyroductSrc; 
  };

  stateCap = pyro.capabilityBuild {
    name = "state";
    version = "0.1.0";
    src = ../capabilities/state;
  };

  stateMod = pyro.moduleBuild {
    name = "cap_state";
    interfaces = [];
    capabilities = [ stateCap ];
    src = ../modules/cap_state;
  };

  # Combine artifacts into a single directory for the test pipeline
  testBundle = pkgs.stdenv.mkDerivation {
    pname = "pyroduct-socket-bundle";
    version = "0.1.0";
    buildPhase = ''
      mkdir -p $out/capabilities/state/0.1.0
      cp -r ${stateCap.drv}/capabilities/state/0.1.0/* $out/capabilities/state/0.1.0/
      
      mkdir -p $out/modules/cap_state
      cp -r ${stateMod.drv}/artifacts/* $out/modules/cap_state/
    '';
  };

  drv = pkgs.stdenv.mkDerivation {
    pname = "pyroduct-socket-test";
    version = "0.1.0";
    src = ./..;

    nativeBuildInputs = [ 
      pkgs.bash 
      pyroduct 
    ];

    doCheck = true;

    checkPhase = ''
      export HOME=$TMPDIR
      SOCKET_PATH="$(pwd)/test.sock"
      
      echo "=== Preparing Pipeline Config ==="
      # The pipeline points to the Nix store paths of our built components
      cat <<EOF > pipeline.toml
      [pipeline]
      name = "socket-test"
      steps = [
        { module = "cap_state", config = {} }
      ]

      [libraries]
      state = { path = "${testBundle}/capabilities/state/0.1.0" }
      
      [modules]
      cap_state = { path = "${testBundle}/modules/cap_state" }
      EOF

      echo '{"input": 10}' > input.jsonl

      echo "--- Testing Unix socket ---"
      # 1. Start server
      pyroduct run pipeline.toml "dummy" --socket "$SOCKET_PATH" > server.log 2>&1 &
      SERVER_PID=$!

      # 2. Poll for the socket
      for i in {1..10}; do
        if [ -S "$SOCKET_PATH" ]; then break; fi
        sleep 0.5
      done

      # 3. Replay
      pyroduct replay input.jsonl "$SOCKET_PATH"
      
      # 4. Cleanup and Verify
      kill $SERVER_PID
      wait $SERVER_PID || true

      if grep -q "10" server.log || grep -q "11" server.log; then
        echo "Unix socket test passed!"
      else
        echo "Error: Expected counter values in server.log"
        cat server.log
        exit 1
      fi
    '';
  };
in
{
  check = drv;
  bin = drv;
}
