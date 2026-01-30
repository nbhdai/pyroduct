# TOML generator from Nix attrsets
# Handles Cargo.toml specific patterns well
{ lib }:

let
  inherit (lib) concatStringsSep mapAttrsToList isAttrs isList isString isBool isInt isFloat hasAttr;

  # Escape a TOML string
  escapeString = s: ''"${builtins.replaceStrings [''"'' "\\" "\n"] [''\"'' "\\\\" "\\n"] s}"'';

  # Check if an attrset should be rendered inline (for dependency specs)
  # Inline if it only contains simple values (strings, bools, numbers, lists of strings)
  shouldInline = v: 
    if !isAttrs v then false
    else lib.all (val: 
      isString val || isBool val || isInt val || isFloat val ||
      (isList val && lib.all isString val)
    ) (lib.attrValues v);

  # Convert a value to TOML (for inline/simple contexts)
  valueToToml = v:
    if isString v then escapeString v
    else if isBool v then (if v then "true" else "false")
    else if isInt v then toString v
    else if isFloat v then toString v
    else if isList v then 
      if lib.length v == 0 then "[]"
      else if lib.all isString v then ''[${concatStringsSep ", " (map escapeString v)}]''
      else "[${concatStringsSep ", " (map valueToToml v)}]"
    else if isAttrs v then inlineTableToToml v
    else throw "Unsupported TOML value type: ${builtins.typeOf v}";

  # Convert an attrset to inline table format { a = 1, b = 2 }
  inlineTableToToml = attrs:
    if attrs == {} then "{}"
    else "{ ${concatStringsSep ", " (mapAttrsToList (k: v: "${k} = ${valueToToml v}") attrs)} }";

  # Render a dependency section specially (Cargo.toml style)
  renderDependencies = deps:
    concatStringsSep "\n" (mapAttrsToList (name: spec:
      if isString spec then ''${name} = ${escapeString spec}''
      else if shouldInline spec then ''${name} = ${inlineTableToToml spec}''
      else ''${name} = ${inlineTableToToml spec}''
    ) deps);

  # Render a features section specially
  renderFeatures = features:
    concatStringsSep "\n" (mapAttrsToList (name: deps:
      ''${name} = [${concatStringsSep ", " (map escapeString deps)}]''
    ) features);

  # Render a section with special handling for known Cargo.toml patterns
  renderSection = path: attrs:
    let
      sectionName = concatStringsSep "." path;
      sectionHeader = if path == [] then "" else "\n[${sectionName}]";
      
      # Special handling for dependencies section
      isDepsSection = lib.last path == "dependencies" || 
                      lib.last path == "dev-dependencies" ||
                      lib.last path == "build-dependencies";
      
      isFeaturesSection = path == ["features"];
      
      content = 
        if isDepsSection then renderDependencies attrs
        else if isFeaturesSection then renderFeatures attrs
        else let
          # Separate simple KVs from nested sections
          simpleKVs = lib.filterAttrs (k: v: !(isAttrs v) || shouldInline v) attrs;
          nestedSections = lib.filterAttrs (k: v: isAttrs v && !shouldInline v) attrs;
          
          kvLines = mapAttrsToList (k: v: 
            if shouldInline v then ''${k} = ${inlineTableToToml v}''
            else ''${k} = ${valueToToml v}''
          ) simpleKVs;
          
          nestedLines = mapAttrsToList (k: v: renderSection (path ++ [k]) v) nestedSections;
        in
          concatStringsSep "\n" ((lib.filter (x: x != "") kvLines) ++ nestedLines);
    in
      if content == "" then sectionHeader
      else sectionHeader + "\n" + content;

  # Main entry point
  toToml = attrs:
    let
      # Process top-level sections in a specific order for Cargo.toml
      orderedKeys = [ "package" "lib" "bin" "features" "dependencies" "dev-dependencies" "build-dependencies" ];
      knownKeys = lib.filter (k: hasAttr k attrs) orderedKeys;
      otherKeys = lib.filter (k: !(lib.elem k orderedKeys)) (lib.attrNames attrs);
      allKeys = knownKeys ++ otherKeys;
      
      sections = map (k: renderSection [k] attrs.${k}) allKeys;
    in
      lib.concatStringsSep "\n" (lib.filter (x: x != "" && x != "\n") sections);

in toToml