{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module";
  src = ./.;
  
  capabilities = [
    capabilities.serial_client
  ];
  
  dependencies = [
    # Add any additional dependencies here
  ];
}