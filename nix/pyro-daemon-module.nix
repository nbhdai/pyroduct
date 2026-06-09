# NixOS module for pyro-daemon
#
# Usage in your NixOS configuration:
#
#   # flake.nix
#   inputs.pyroduct.url = "github:nbhdai/pyroduct";
#
#   # configuration.nix
#   imports = [ pyroduct.nixosModules.pyro-daemon ];
#
#   services.pyro-daemon = {
#     enable = true;
#     members = [ "sven" ];       # users who can access the daemon & cache
#     authorName = "nbhdai";
#     buildSlots = 4;
#   };
#

{ config, lib, pkgs, ... }:

let
  cfg = config.services.pyro-daemon;

  cacheConfigFile = pkgs.writeText "pyroduct-config.toml" ''
    author = "${cfg.authorName}"
    build_slots = ${toString cfg.buildSlots}
  '';
in
{
  # ===========================================================================
  # Module options
  # ===========================================================================
  options.services.pyro-daemon = {
    enable = lib.mkEnableOption "Pyroduct daemon (pyro-daemond)";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The pyroduct package to use (must provide pyro-daemond binary).";
    };

    authorName = lib.mkOption {
      type = lib.types.str;
      default = "pyroduct";
      description = "Author name written to the shared cache config.toml.";
    };

    buildSlots = lib.mkOption {
      type = lib.types.int;
      default = 4;
      description = "Number of build slots written to the shared cache config.toml.";
    };

    members = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        List of users to add to the 'pyroduct' group.
        Members can access the shared cache and connect to the daemon socket.
      '';
    };
  };

  # ===========================================================================
  # Module implementation
  # ===========================================================================
  config = lib.mkIf cfg.enable {

    # -------------------------------------------------------------------------
    # System user & group
    # -------------------------------------------------------------------------
    users.users.pyroduct = {
      isSystemUser = true;
      group = "pyroduct";
      home = "/var/lib/pyro-daemon";
      description = "Pyroduct daemon service user";
    };

    users.groups.pyroduct = {
      members = cfg.members;
    };

    # -------------------------------------------------------------------------
    # Directory setup via tmpfiles
    # -------------------------------------------------------------------------
    systemd.tmpfiles.rules = [
      # Daemon working directory — pyroduct:pyroduct, 0750
      "d /var/lib/pyro-daemon        0750 pyroduct pyroduct -"
      "d /var/lib/pyro-daemon/data   0750 pyroduct pyroduct -"

      # Shared cache — setgid (2775) so group members can write
      "d /var/lib/pyro-daemon/cache              2775 pyroduct pyroduct -"
      "d /var/lib/pyro-daemon/cache/capabilities  2775 pyroduct pyroduct -"
      "d /var/lib/pyro-daemon/cache/interfaces    2775 pyroduct pyroduct -"
      "d /var/lib/pyro-daemon/cache/modules       2775 pyroduct pyroduct -"

      # Cache config file
      "C /var/lib/pyro-daemon/cache/config.toml  0664 pyroduct pyroduct - ${cacheConfigFile}"
    ];

    # -------------------------------------------------------------------------
    # Systemd service
    # -------------------------------------------------------------------------
    systemd.services.pyro-daemon = {
      description = "Pyroduct Daemon - Background Playbook and Process Supervisor";
      documentation = [ "https://github.com/nbhdai/pyroduct" ];
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      environment = {
        PYRODUCT = "/var/lib/pyro-daemon/cache";
        PYRO_DAEMON_DIR = "/var/lib/pyro-daemon";
      };

      serviceConfig = {
        Type = "simple";
        User = "pyroduct";
        Group = "pyroduct";

        ExecStart = "${cfg.package}/bin/pyro-daemond --working-dir /var/lib/pyro-daemon";

        # State
        StateDirectory = "pyro-daemon";
        WorkingDirectory = "/var/lib/pyro-daemon";

        # Group-accessible socket & files
        UMask = "0007";

        # Restart policy
        Restart = "on-failure";
        RestartSec = 5;

        # Sandboxing: daemon only accesses /var/lib/pyro-daemon
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        NoNewPrivileges = true;
        ReadWritePaths = [ "/var/lib/pyro-daemon" ];
      };
    };

    # -------------------------------------------------------------------------
    # Environment variables for interactive users
    # -------------------------------------------------------------------------
    environment.etc."profile.d/pyroduct.sh" = {
      text = ''
        # Pyroduct environment variables
        export PYRODUCT="/var/lib/pyro-daemon/cache"
        export PYRO_DAEMON_DIR="/var/lib/pyro-daemon"
      '';
      mode = "0644";
    };

    # -------------------------------------------------------------------------
    # Make the pyroduct CLI available system-wide
    # -------------------------------------------------------------------------
    environment.systemPackages = [ cfg.package ];
  };
}
