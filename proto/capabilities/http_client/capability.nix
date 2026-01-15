{ myLib }:

myLib.buildCapability {
  name = "proto_http_client";
  src = ./.;
  
  # Dependencies only needed on the host side (native plugin)
  hostDependencies = [
    { name = "reqwest"; version = "0.13.1"; features = [ "json" ]; }
  ];
  
  # Dependencies only needed on the module side (wasm)
  moduleDependencies = [];
  
  # Dependencies needed on both sides
  sharedDependencies = [
    { name = "rkyv"; version = "0.8"; features = [ "std" ]; }
  ];
}