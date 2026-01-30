{ myLib, capabilities }:

myLib.buildModule {
  name = "proto_module";
  src = ./.;
  
  capabilities = [
    capabilities.proto_reporter
  ];
  
  dependencies = [
    { name = "text_splitters"; version = "0.15"; }
  ];
}