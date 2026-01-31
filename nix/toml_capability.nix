{ lib, toToml, mkDep, pyroductDep }:

{
  name,
  version ? "0.1.0",
  hostDependencies ? [],
  moduleDependencies ? [],
  sharedDependencies ? [],
  extraCargoToml ? {},
  authors ? [ "Sven Cattell" ],
  edition ? "2024",
  ...
}: 

let
  # --- Dependency Formatting ---
  formatDep = dep: 
    if builtins.isAttrs dep then dep
    # Check if string looks like a path (starts with . or /)
    else if builtins.isString dep then
      if (builtins.substring 0 1 dep == ".") || (builtins.substring 0 1 dep == "/") then
        # It is a path string: "../thingie"
        # We infer the name from the filename: name = "thingie", path = "../thingie"
        { name = baseNameOf dep; path = dep; }
      else
        # It is a crate name: "serde"
        { name = dep; version = "*"; }
    else throw "Invalid dependency format: ${builtins.typeOf dep}";

  hostDeps = map formatDep hostDependencies;
  moduleDeps = map formatDep moduleDependencies;
  sharedDeps = map formatDep sharedDependencies;

  hostFeatureList = map (d: "dep:${d.name}") hostDeps;
  moduleFeatureList = map (d: "dep:${d.name}") moduleDeps;

  # --- Generate Cargo.toml Structure ---
  cargoToml = {
    package = {
      inherit name version edition authors;
    };
    lib = { crate-type = [ "cdylib" "rlib" ]; };
    features = {
      default = [];
      capability = hostFeatureList;
      module = moduleFeatureList;
    };
    dependencies = lib.listToAttrs (
      (map (d: { name = d.name; value = mkDep d; }) sharedDeps) ++
      (map (d: { name = d.name; value = mkDep (d // { optional = true; }); }) hostDeps) ++
      (map (d: { name = d.name; value = mkDep (d // { optional = true; }); }) moduleDeps)
    ) // {
      # FIX: Discard string context so this path can be written to a pure TOML string
      pyroduct = pyroductDep;
    };
  } // extraCargoToml;

in
  toToml cargoToml