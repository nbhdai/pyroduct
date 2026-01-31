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
    if builtins.isAttrs dep then dep
    else if builtins.isString dep then
      if (builtins.substring 0 1 dep == ".") || (builtins.substring 0 1 dep == "/") then
        { name = baseNameOf dep; path = dep; }
      else
        { name = dep; version = "*"; }
    else throw "Invalid dependency format: ${builtins.typeOf dep}";
    
  deps = map formatDep dependencies;
  
  # Process capabilities: Enforce "module" feature and name inference
  processCap = cap:
    let
      formatted = formatDep cap;
      
      # Name inference: Use provided name, or derive from path
      capName = formatted.name or (
        if formatted ? path then baseNameOf formatted.path 
        else throw "Capability dependency must have a 'name' or be a path with a filename."
      );
      
      # Error checking: User cannot provide features
      _ = if formatted ? features then 
        throw "Capability '${capName}' defines 'features', which is not allowed. Capabilities are automatically assigned features = [\"module\"]." 
        else null;
    in
    {
      name = capName;
      value = mkDep (formatted // {
        name = capName;
        features = [ "module" ];
      });
    };

  # --- Generate Cargo.toml Content ---
  cargoToml = {
    package = {
      inherit name version edition authors;
    };
    lib = { crate-type = [ "cdylib" ]; };
    dependencies = lib.listToAttrs (
      # Regular dependencies
      (map (d: { name = d.name; value = mkDep d; }) deps) ++
      # Capability dependencies (Processed)
      (map processCap capabilities)
    ) // {
      # FIX: Discard string context for pyroduct path if it's a store path
      pyroduct = pyroductDep;
      tracing = "*";
    };
  } // extraCargoToml;

in
  toToml cargoToml