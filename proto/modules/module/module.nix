{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module";
  src = ./.;
  
  capabilities = [
    capabilities.proto_reporter
  ];
  
  dependencies = [
    # Add any additional dependencies here
  ];
}