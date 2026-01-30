{ myLib }:

myLib.buildCapability {
  name = "cpu_client";
  src = ./.;
  
  # Dependencies only needed on the host side (native plugin)
  hostDependencies = [];
  
  # Dependencies only needed on the module side (wasm)
  moduleDependencies = [];
  
  # Dependencies needed on both sides
  sharedDependencies = [];
}