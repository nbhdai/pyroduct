{ lib, toToml, mkDep, pyroductDep }:

{
  name,
  version ? "0.1.0",
  capabilities ? [],
  dependencies ? [],
  extraCargoToml ? {},
  authors ? [ "Sven Cattell" ],
  edition ? "2024",
  ...
}: 

let
  formatDep = dep: 
    if builtins.isString dep then { name = dep; version = "*"; }
    else if builtins.isAttrs dep then dep
    else throw "Invalid dependency format";
    
  deps = map formatDep dependencies;

  # --- Generate Cargo.toml Content ---
  cargoToml = {
    package = {
      inherit name version edition authors;
    };
    lib = { crate-type = [ "cdylib" ]; };
    dependencies = lib.listToAttrs (
      # Regular dependencies
      (map (d: { name = d.name; value = mkDep d; }) deps) ++
      # Capability dependencies
      (map (cap: { 
        name = cap.pname or cap.name; 
        value = mkDep {
          name = cap.pname or cap.name;
          # FIX: Discard string context for capability paths
          path = builtins.unsafeDiscardStringContext "${cap.output}/crate";
          features = [ "module" ];
        }; 
      }) capabilities)
    ) // {
      # FIX: Discard string context for pyroduct path
      pyroduct = pyroductDep;
      tracing = "*";
    };
  } // extraCargoToml;

in
  toToml cargoToml