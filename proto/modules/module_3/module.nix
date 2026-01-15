{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module_3";
  src = ./.;
  
  capabilities = [
    capabilities.proto_serial_client
    capabilities.proto_reporter
  ];
  
  dependencies = [
    { name = "rkyv"; version = "0.8"; features = [ "std" ]; }
  ];
}