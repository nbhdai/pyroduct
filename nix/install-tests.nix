{ pkgs, pyroduct }:

let
  # =========================================================================
  # NixOS VM test: validates the pyro-daemon NixOS module
  # =========================================================================
  nixosTest = pkgs.nixosTest {
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
  # Uses pre-built binaries (stubbed) to test the setup logic
  # =========================================================================
  installScriptTest = pkgs.nixosTest {
    name = "pyro-daemon-install-script-test";

    nodes.machine = { pkgs, ... }: {
      environment.systemPackages = with pkgs; [
        bash
        coreutils
        gnugrep
        gnused
        gawk
        systemd
        shadow   # useradd, usermod, etc.
        curl
        file
      ];

      # The install script needs a user to run as
      users.users.installer = {
        isNormalUser = true;
        password = "test";
        extraGroups = [ "wheel" ];
      };

      security.sudo.wheelNeedsPassword = false;
    };

    testScript = let
      # Create a minimal fake repo with stubbed binaries
      fakeRepo = pkgs.runCommand "fake-pyroduct-repo" {} ''
        mkdir -p $out/lib/pyroduct $out/lib/pyro-daemon

        # Create stub Cargo.tomls so the script thinks it's a repo
        echo '[package]' > $out/lib/pyroduct/Cargo.toml
        echo 'name = "pyroduct"' >> $out/lib/pyroduct/Cargo.toml
        echo '[package]' > $out/lib/pyro-daemon/Cargo.toml
        echo 'name = "pyro-daemon"' >> $out/lib/pyro-daemon/Cargo.toml

        # Copy the install script
        cp ${../install.sh} $out/install.sh
        chmod +x $out/install.sh

        # Create stub binaries that the script will "find"
        mkdir -p $out/stub-bin
        echo '#!/bin/sh' > $out/stub-bin/pyro-daemond
        echo 'echo "pyro-daemond stub"' >> $out/stub-bin/pyro-daemond
        chmod +x $out/stub-bin/pyro-daemond
        echo '#!/bin/sh' > $out/stub-bin/pyroduct
        echo 'echo "pyroduct stub"' >> $out/stub-bin/pyroduct
        chmod +x $out/stub-bin/pyroduct
        echo '#!/bin/sh' > $out/stub-bin/cargo
        echo 'echo "cargo install stub — skipping"' >> $out/stub-bin/cargo
        chmod +x $out/stub-bin/cargo
        echo '#!/bin/sh' > $out/stub-bin/rustc
        echo 'echo "rustc 1.90.0 (stub)"' >> $out/stub-bin/rustc
        chmod +x $out/stub-bin/rustc
      '';
    in ''
      machine.wait_for_unit("multi-user.target")

      # Copy fake repo and put stub binaries on PATH
      machine.succeed("cp -r ${fakeRepo} /tmp/pyroduct && chmod -R u+w /tmp/pyroduct")
      machine.succeed("cp ${fakeRepo}/stub-bin/* /usr/local/bin/ 2>/dev/null || cp ${fakeRepo}/stub-bin/* /run/current-system/sw/bin/ 2>/dev/null || true")
      machine.succeed("export PATH='${fakeRepo}/stub-bin:$PATH' && cd /tmp/pyroduct && ./install.sh -d")

      # Verify the pyroduct system user was created
      machine.succeed("id pyroduct")

      # Verify directory structure
      machine.succeed("test -d /var/lib/pyro-daemon")
      machine.succeed("test -d /var/lib/pyro-daemon/cache")
      machine.succeed("test -f /var/lib/pyro-daemon/cache/config.toml")

      # Verify environment files
      machine.succeed("test -f /etc/pyroduct.env")
      machine.succeed("test -f /etc/profile.d/pyroduct.sh")
      machine.succeed("grep -q '/var/lib/pyro-daemon/cache' /etc/pyroduct.env")

      # Verify systemd service file was created
      machine.succeed("test -f /etc/systemd/system/pyro-daemon.service")
      machine.succeed("grep -q 'User=pyroduct' /etc/systemd/system/pyro-daemon.service")
      machine.succeed("grep -q 'ProtectSystem=strict' /etc/systemd/system/pyro-daemon.service")
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
