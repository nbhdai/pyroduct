{ pkgs, pyroduct, pyro-daemon }:

let
  # =========================================================================
  # NixOS VM test: validates the pyro-daemon NixOS module
  # =========================================================================
  nixosTest = pkgs.testers.nixosTest {
    name = "pyro-daemon-nixos-test";

    nodes.machine = { ... }: {
      imports = [ ./pyro-daemon-module.nix ];

      services.pyro-daemon = {
        enable = true;
        package = pyro-daemon;
        cliPackage = pyroduct;
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

in
{
  check = nixosTest;
}
