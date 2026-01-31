{ myLib }:

myLib.buildModule {
  name = "basic_capability";
  src = ./.;
  
  capabilities = [
    { path = "../../capabilities/serial_client"; }
  ];
  
  dependencies = [];
}