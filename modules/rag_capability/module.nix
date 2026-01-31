{ myLib }:

myLib.buildModule {
  name = "rag_capability";
  src = ./.;
  
  capabilities = [
    { path = "../../capabilities/rag"; }
  ];
  
  dependencies = [
    { name = "text_splitters"; version = "0.15"; }
  ];
}