use cargo_toml::{
    Badges, Dependency, DependencyDetail, DepsSet, FeatureSet, Inheritable, LintGroups, Manifest, Package, PatchSet, Product, Profiles, TargetDepsSet, Workspace
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::Value;

// ============================================================================
// 1. THE CUSTOM SCHEMA (Exactly as you requested)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CapabilityManifest<Metadata = Value> {
    pub package: Option<Package<Metadata>>,
    pub workspace: Option<Workspace<Metadata>>,

    #[serde(default)]
    pub dependencies: CapabilityDependencies,

    #[serde(default)]
    pub dev_dependencies: DepsSet,
    #[serde(default)]
    pub build_dependencies: DepsSet,
    #[serde(default)]
    pub target: TargetDepsSet,
    #[serde(default)]
    pub features: FeatureSet,
    
    #[serde(default)]
    #[deprecated(note = "Cargo recommends patch instead")]
    pub replace: DepsSet,
    #[serde(default)]
    pub patch: PatchSet,
    
    pub lib: Option<Product>,
    #[serde(default)]
    pub profile: Profiles,
    #[serde(default)]
    pub badges: Badges,
    #[serde(default)]
    pub bin: Vec<Product>,
    #[serde(default)]
    pub bench: Vec<Product>,
    #[serde(default)]
    pub test: Vec<Product>,
    #[serde(default)]
    pub example: Vec<Product>,
    #[serde(default)]
    pub lints: Inheritable<LintGroups>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CapabilityDependencies {
    #[serde(default)]
    pub host: DepsSet,
    #[serde(default)]
    pub module: DepsSet,
    #[serde(default)]
    pub shared: DepsSet,
}

// ============================================================================
// 2. LOGIC TO AUGMENT & CONVERT
// ============================================================================

impl CapabilityManifest {
    /// Reads from a file string, processes logic, and returns a standard Manifest 
    /// ready for serialization.
    pub fn to_cargo_manifest(self, pyroduct: Dependency) -> Manifest {
        let mut final_deps = BTreeMap::new();
        final_deps.extend(self.dependencies.shared.clone().into_iter());
        
        // 1. Augment and merge dependencies
        // Host: Optional = true
        self.augment_deps(&mut final_deps, &self.dependencies.host, true);
        
        // Module: Optional = true
        self.augment_deps(&mut final_deps, &self.dependencies.module, true);

        // Always add pyroduct workspace dep
        final_deps.insert("pyroduct".to_string(), pyroduct);

        // 2. Create Requisite Features
        let final_features = self.create_requisite_features(&self.features);

        // 3. Construct Standard Manifest
        // We move fields over. Note: Manifest uses `Option` for some fields 
        // where your struct might rely on defaults, so we map carefully.
        #[allow(deprecated)]
        Manifest {
            package: self.package,
            workspace: self.workspace,
            dependencies: final_deps,
            dev_dependencies: self.dev_dependencies,
            build_dependencies: self.build_dependencies,
            target: self.target,
            features: final_features,
            patch: self.patch,
            lib: self.lib,
            profile: self.profile,
            badges: self.badges,
            bin: self.bin,
            bench: self.bench,
            test: self.test,
            example: self.example,
            lints: self.lints,
            replace: BTreeMap::default(),
        }
    }

    /// Helper: Augments dependencies with `optional = true` if requested
    /// and inserts them into the final map.
    fn augment_deps(
        &self, 
        target_map: &mut DepsSet, 
        source_map: &DepsSet, 
        make_optional: bool
    ) {
        for (name, dep) in source_map {
            let new_dep = if make_optional {
                match dep {
                    // Convert Simple ("1.0") -> Detailed { version = "1.0", optional = true }
                    Dependency::Simple(ver) => Dependency::Detailed(Box::new(DependencyDetail {
                        version: Some(ver.clone()),
                        optional: true,
                        ..Default::default()
                    })),
                    // Update Detailed to ensure optional is true
                    Dependency::Detailed(detail) => {
                        let mut d = detail.clone();
                        d.optional = true;
                        Dependency::Detailed(d)
                    },
                    // Inherited workspace deps also need to become detailed to hold the optional flag
                    Dependency::Inherited(inherited) => {
                        let mut d = inherited.clone();
                        d.optional = true;
                        Dependency::Inherited(d)
                    }
                }
            } else {
                dep.clone()
            };
            target_map.insert(name.clone(), new_dep);
        }
    }

    /// Helper: Generates the "capability" feature and defaults
    fn create_requisite_features(&self, existing_features: &FeatureSet) -> FeatureSet {
        let mut new_features = existing_features.clone();

        // Generate "dep:xxx" entries for all Host dependencies
        let capability_feature: Vec<String> = self.dependencies.host.keys()
            .map(|name| format!("dep:{}", name))
            .collect();

        let module_feature: Vec<String> = self.dependencies.module.keys()
            .map(|name| format!("dep:{}", name))
            .collect();

        new_features.insert("capability".to_string(), capability_feature);
        new_features.insert("module".to_string(), module_feature);
        
        // Ensure default and module exist (if not provided in input)
        new_features.entry("default".to_string()).or_default();

        new_features
    }
}

// ============================================================================
// 3. USAGE EXAMPLE / TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use cargo_toml::InheritedDependencyDetail;

    use super::*;

    #[test]
    fn test_full_transformation() {
        let input_toml = r#"
[package]
name = "my-capability"
version = "0.1.0"
edition = "2021"
authors = ["Me"]

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies.host]
tokio = "1.0"
uuid = { version = "1.0", features = ["v4"] }

[dependencies.module]
wasm-bindgen = "0.2"

[dependencies.shared]
serde = { version = "1.0", features = ["derive"] }
"#;

        // 1. Deserialize into Custom Struct
        let cap_manifest: CapabilityManifest = toml::from_str(input_toml).unwrap();
        let pyroduct = Dependency::Inherited(InheritedDependencyDetail { workspace: true, ..Default::default() });

        // 2. Convert to Standard Manifest (Augment logic runs here)
        let standard_manifest = cap_manifest.to_cargo_manifest(pyroduct);

        // 3. Verify Dependencies
        let deps = &standard_manifest.dependencies;
        
        // Check Host (converted to optional)
        match deps.get("tokio").unwrap() {
            Dependency::Detailed(d) => assert_eq!(d.optional, true),
            _ => panic!("tokio should be detailed"),
        }
        
        // Check Shared (remains not optional)
        match deps.get("serde").unwrap() {
            Dependency::Detailed(d) => assert_eq!(d.optional, false),
            _ => panic!("serde should be detailed"),
        }

        // Check Pyroduct added
        assert!(deps.contains_key("pyroduct"));

        // 4. Verify Features
        let features = &standard_manifest.features;
        let cap_feat = features.get("capability").unwrap();
        
        assert!(cap_feat.contains(&"dep:tokio".to_string()));
        assert!(cap_feat.contains(&"dep:uuid".to_string()));
        // Module deps should NOT be in capability feature
        assert!(!cap_feat.contains(&"dep:wasm-bindgen".to_string())); 

        // 5. Serialize back to String
        let output = toml::to_string_pretty(&standard_manifest).unwrap();
        println!("{}", output);
    }
}