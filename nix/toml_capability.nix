{ lib, toToml, mkDep, pyroductPath }:

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
    if builtins.isString dep then { name = dep; version = "*"; }
    else if builtins.isAttrs dep then dep
    else throw "Invalid dependency format";

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
      pyroduct = { path = builtins.unsafeDiscardStringContext pyroductPath; };
    };
  } // extraCargoToml;

in
  toToml cargoToml