{ myLib }:

myLib.buildModule {
  name = "basic";
  src = ./.;
  
  capabilities = [
    # This doesn't link to any capabilities
  ];
  
  dependencies = [
    # Add any additional dependencies here
  ];
}