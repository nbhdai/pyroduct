{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module";
  src = ./.;
  
  capabilities = [
    # This doesn't link to any capabilities
  ];
  
  dependencies = [
    # Add any additional dependencies here
  ];
}