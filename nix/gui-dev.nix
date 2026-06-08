{ pkgs, process-compose-flake }:

let
  processComposeLib = import process-compose-flake.lib { inherit pkgs; };
in
processComposeLib.makeProcessCompose {
  modules = [
    {
      settings.processes = {
        daemon = {
          command = "${pkgs.bacon}/bin/bacon --job daemon --headless";
          working_dir = "lib";
          environment = {
            PYRO_DAEMON_DIR = "../test";
            PYRODUCT = "../test";
          };
        };
        tauri = {
          command = "${pkgs.cargo-tauri}/bin/cargo-tauri dev";
          working_dir = "lib/pyro-gui";
          environment = {
            PYRO_DAEMON_DIR = "../../test";
            PYRODUCT = "../../test";
          };
        };
      };
    }
  ];
}
