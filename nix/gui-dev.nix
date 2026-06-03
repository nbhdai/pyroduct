{ pkgs, process-compose-flake }:

let
  processComposeLib = import process-compose-flake.lib { inherit pkgs; };
in
processComposeLib.makeProcessCompose {
  modules = [
    {
      settings.processes = {
        daemon = {
          command = "${pkgs.bacon}/bin/bacon --job daemon";
          working_dir = "lib";
        };
        tauri = {
          command = "${pkgs.cargo-tauri}/bin/cargo-tauri dev";
          working_dir = "lib/pyro-gui";
        };
      };
    }
  ];
}
