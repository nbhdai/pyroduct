{ myLib }:

myLib.buildCapability {
  name = "serial_client";
  src = ./.;
  
  # Dependencies only needed on the host side (native plugin)
  hostDependencies = [
    { name = "tokio"; version = "1.49.0"; features = ["full"]; }
    { name = "tokio-serial"; version = "5.4.1"; }
  ];
  
  # Dependencies only needed on the module side (wasm)
  moduleDependencies = [];
  
  # Dependencies needed on both sides
  sharedDependencies = [];
}