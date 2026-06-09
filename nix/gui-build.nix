{
  pkgs,
  process-compose-flake,
  pyro-daemon,
  pyro-gui,
  workingDir ? "./test",
  configSettings ? {
    target = "../target";
    author = "test_anon";
    pyroduct = {
      path = "../lib/pyroduct";
    };
    build_slots = 4;
  },
}:

let
  processComposeLib = import process-compose-flake.lib { inherit pkgs; };
  configFile = pkgs.writeText "pyroduct-config.toml" (pkgs.lib.generators.toTOML { } configSettings);
in
processComposeLib.makeProcessCompose {
  modules = [
    {
      settings.processes = {
        setup = {
          command = "mkdir -p ${workingDir} && cp ${configFile} ${workingDir}/config.toml && chmod u+w ${workingDir}/config.toml";
        };
        daemon = {
          command = "${pyro-daemon}/bin/pyro-daemond";
          depends_on = {
            setup = {
              condition = "process_completed_successfully";
            };
          };
          environment = {
            PYRO_DAEMON_DIR = workingDir;
            PYRODUCT = workingDir;
          };
        };
        tauri = {
          command = "${pyro-gui}/bin/pyro-gui";
          depends_on = {
            setup = {
              condition = "process_completed_successfully";
            };
          };
          environment = {
            PYRO_DAEMON_DIR = workingDir;
            PYRODUCT = workingDir;
          };
        };
      };
    }
  ];
}
