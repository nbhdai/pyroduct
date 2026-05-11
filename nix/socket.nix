{ pkgs }:

let
  drv = pkgs.stdenv.mkDerivation {
  pname = "pyroduct-socket-test";
  version = "0.1.0";
  src = ./..;

  nativeBuildInputs = [ 
    pkgs.bash 
    pkgs.nettools # for netstat if needed
    # Ensure pyroduct is actually here!
    pkgs.pyroduct 
  ];

  doCheck = true; # Required to trigger checkPhase

  checkPhase = ''
    export HOME=$TMPDIR
    
    # Use the current directory for the socket to avoid absolute path issues
    SOCKET_PATH="$(pwd)/test.sock"
    
    echo "=== Preparing Test Data ==="
    cat << 'EOF' > pipeline.toml
    [pipeline]
    name = "test-pipeline"
    steps = []
    EOF

    echo '{"test": "row1"}' > input.jsonl

    echo "--- Testing Unix socket ---"
    # 1. Start server
    pyroduct run pipeline.toml "dummy" --socket "$SOCKET_PATH" > server.log 2>&1 &
    SERVER_PID=$!

    # 2. Poll for the socket (don't just sleep; it's flaky)
    for i in {1..10}; do
      if [ -S "$SOCKET_PATH" ]; then break; fi
      sleep 0.5
    done

    # 3. Replay
    pyroduct replay input.jsonl "$SOCKET_PATH"
    
    # 4. Cleanup and Verify
    kill $SERVER_PID
    wait $SERVER_PID || true # Ignore exit code from kill

    if grep -q "Pipeline Succeeded!" server.log; then
      echo "Unix socket test passed!"
    else
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
