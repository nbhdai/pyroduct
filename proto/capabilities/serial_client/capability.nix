{ myLib }:

myLib.buildCapability {
  name = "proto_serial_client";
  src = ./.;
  
  # Dependencies only needed on the host side (native plugin)
  hostDependencies = [
    { name = "tokio"; version = "1.0"; features = [ "full" ]; }
    { name = "tokio-serial"; version = "5.4"; }
    { name = "serde"; version = "1.0"; features = [ "derive" ]; }
  ];
  
  # Dependencies only needed on the module side (wasm)
  moduleDependencies = [];
  
  # Dependencies needed on both sides
  sharedDependencies = [
    { name = "rkyv"; version = "0.8"; features = [ "std" ]; }
  ];
}