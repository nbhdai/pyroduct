# proto/capabilities/reporter/capability.nix
{ myLib }:

myLib.buildCapability {
  name = "proto_reporter";
  src = ./.;
  
  # Dependencies only needed on the host side (native plugin)
  hostDependencies = [
    { name = "serde"; version = "1.0"; features = [ "derive" ]; }
    { name = "serde_json"; version = "1.0"; }
  ];
  
  # Dependencies only needed on the module side (wasm)
  moduleDependencies = [
    # None for this capability
  ];
  
  # Dependencies needed on both sides
  sharedDependencies = [
    { name = "rkyv"; version = "0.8"; features = [ "std" ]; }
  ];
}