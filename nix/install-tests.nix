{ pkgs, pyroduct }:

let
  # =========================================================================
  # Install script test: validates install.sh in a minimal NixOS VM
  # Uses a pre-built stub binary to test the setup logic
  # =========================================================================
  installScriptTest = pkgs.testers.nixosTest {
    name = "pyroduct-install-script-test";

    nodes.machine = { ... }: {
      environment.systemPackages = with pkgs; [
        bash
        coreutils
        gnugrep
        gnused
        gawk
      ];

      # The install script needs a user to run as
      users.users.installer = {
        isNormalUser = true;
        password = "test";
        shell = pkgs.bash;
      };
    };

    testScript = let
      # Create a fake release tarball directory with a stub pyroduct binary
      fakeRelease = pkgs.runCommand "fake-pyroduct-release" {} ''
        mkdir -p $out

        # Create a stub pyroduct binary (simulates pre-built release binary)
        echo '#!/bin/sh' > $out/pyroduct
        echo 'echo "pyroduct 0.2.1 (stub)"' >> $out/pyroduct
        chmod +x $out/pyroduct

        # Copy the install script
        cp ${../install.sh} $out/install.sh
        chmod +x $out/install.sh
      '';
    in ''
      machine.wait_for_unit("multi-user.target")

      # Copy fake release to a writable location and run as installer user
      machine.succeed("cp -r ${fakeRelease} /tmp/pyroduct-release && chmod -R u+w /tmp/pyroduct-release")
      machine.succeed("chown -R installer:users /tmp/pyroduct-release")

      # Run install script as the installer user
      machine.succeed("su - installer -c 'cd /tmp/pyroduct-release && ./install.sh'")

      # Verify the pyroduct binary was installed
      machine.succeed("su - installer -c 'test -x /home/installer/.local/bin/pyroduct'")
      machine.succeed("su - installer -c '/home/installer/.local/bin/pyroduct --version'")

      # Verify ~/.pyroduct directory was created
      machine.succeed("su - installer -c 'test -d /home/installer/.pyroduct'")
      machine.succeed("su - installer -c 'test -f /home/installer/.pyroduct/config.toml'")
      machine.succeed("su - installer -c 'grep -q installer /home/installer/.pyroduct/config.toml'")
      machine.succeed("su - installer -c 'grep -q build_slots /home/installer/.pyroduct/config.toml'")

      # Verify environment variables were added to shell rc
      machine.succeed("su - installer -c 'grep -q PYRODUCT /home/installer/.bashrc'")
      machine.succeed("su - installer -c 'grep -q PYRO_DAEMON_DIR /home/installer/.bashrc'")
      machine.succeed("su - installer -c 'grep -q .pyroduct /home/installer/.bashrc'")
    '';
  };

  # =========================================================================
  # CI helper script
  # =========================================================================
  testInstallScript = pkgs.writeShellScriptBin "test-install" ''
    set -e
    echo "=== Running install script test ==="
    nix build .#checks.${pkgs.system}.install-script-test --print-build-logs
    echo "=== Install script test passed ==="
  '';

in
{
  install-script-check = installScriptTest;
  bin = testInstallScript;
}
