{ myLib }:

myLib.buildModule {
  name = "rag_capability";
  src = ./.;
  
  capabilities = [
    { path = "../../capabilities/rag"; }
  ];
  
  dependencies = [
    { name = "text-splitter"; version = "0.15"; }
  ];
}