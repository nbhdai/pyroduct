{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module_2";
  src = ./.;
  
  capabilities = [
    capabilities.proto_cpu_info
    capabilities.proto_http_client
  ];
  
  dependencies = [
    { name = "rkyv"; version = "0.8"; features = [ "std" ]; }
  ];
}