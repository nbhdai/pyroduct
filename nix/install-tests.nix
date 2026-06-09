{ pkgs, pyroduct }:

let
  # =========================================================================
  # NixOS VM test: validates the pyro-daemon NixOS module
  # =========================================================================
  nixosTest = pkgs.testers.nixosTest {
    name = "pyro-daemon-nixos-test";

    nodes.machine = { pkgs, ... }: {
      imports = [ ./pyro-daemon-module.nix ];

      services.pyro-daemon = {
        enable = true;
        package = pyroduct;
        members = [ "testuser" ];
        authorName = "ci-test";
        buildSlots = 2;
      };

      users.users.testuser = {
        isNormalUser = true;
        password = "test";
      };
    };

    testScript = ''
      machine.wait_for_unit("multi-user.target")

      # Verify the pyroduct user and group exist
      machine.succeed("id pyroduct")
      machine.succeed("groups testuser | grep -q pyroduct")

      # Verify directory structure
      machine.succeed("test -d /var/lib/pyro-daemon")
      machine.succeed("test -d /var/lib/pyro-daemon/cache")
      machine.succeed("test -d /var/lib/pyro-daemon/cache/capabilities")
      machine.succeed("test -d /var/lib/pyro-daemon/cache/interfaces")
      machine.succeed("test -d /var/lib/pyro-daemon/cache/modules")

      # Verify cache config.toml
      machine.succeed("test -f /var/lib/pyro-daemon/cache/config.toml")
      machine.succeed("grep -q 'ci-test' /var/lib/pyro-daemon/cache/config.toml")
      machine.succeed("grep -q 'build_slots = 2' /var/lib/pyro-daemon/cache/config.toml")

      # Verify ownership and permissions
      machine.succeed("stat -c '%U:%G' /var/lib/pyro-daemon | grep -q 'pyroduct:pyroduct'")
      machine.succeed("stat -c '%U:%G' /var/lib/pyro-daemon/cache | grep -q 'pyroduct:pyroduct'")

      # Verify the systemd service is running
      machine.wait_for_unit("pyro-daemon.service")
      machine.succeed("systemctl is-active pyro-daemon.service")

      # Verify the control socket is created
      machine.wait_until_succeeds("test -S /var/lib/pyro-daemon/control", timeout=10)

      # Verify environment variables are set via profile.d
      machine.succeed("test -f /etc/profile.d/pyroduct.sh")
      machine.succeed("grep -q 'PYRODUCT' /etc/profile.d/pyroduct.sh")
      machine.succeed("grep -q 'PYRO_DAEMON_DIR' /etc/profile.d/pyroduct.sh")

      # Verify the pyroduct CLI is available
      machine.succeed("pyroduct --version")

      # Verify testuser can read the cache (group permission check)
      machine.succeed("su - testuser -c 'cat /var/lib/pyro-daemon/cache/config.toml'")
    '';
  };

  # =========================================================================
  # Install script test: validates install.sh in a minimal NixOS VM
  # Uses a pre-built stub binary to test the setup logic
  # =========================================================================
  installScriptTest = pkgs.testers.nixosTest {
    name = "pyro-daemon-install-script-test";

    nodes.machine = { pkgs, ... }: {
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
  # CI helper scripts
  # =========================================================================
  testInstallScript = pkgs.writeShellScriptBin "test-install" ''
    set -e
    echo "=== Running installer tests ==="

    echo ""
    echo "--- NixOS module test ---"
    nix build .#checks.${pkgs.system}.nixos-module-test --print-build-logs

    echo ""
    echo "--- Install script test ---"
    nix build .#checks.${pkgs.system}.install-script-test --print-build-logs

    echo ""
    echo "=== All installer tests passed ==="
  '';

in
{
  nixos-module-check = nixosTest;
  install-script-check = installScriptTest;
  bin = testInstallScript;
}
