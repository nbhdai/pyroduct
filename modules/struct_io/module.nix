{ myLib }:

myLib.buildModule {
  name = "struct_io";
  src = ./.;
  
  capabilities = [
    { path = "../../capabilities/http_client"; }
  ];
  
  dependencies = [
    # Add any additional dependencies here
  ];
}