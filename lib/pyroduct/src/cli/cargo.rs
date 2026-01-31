use cargo_toml::{
    Badges, Dependency, DependencyDetail, DepsSet, Edition, FeatureSet, Inheritable, InheritedDependencyDetail, LintGroups, Manifest, Package, PatchSet, Product, Profiles, TargetDepsSet, Workspace
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CapabilityManifest<Metadata = Value> {
    pub capability: Option<Package<Metadata>>,
    pub workspace: Option<Workspace<Metadata>>,
    #[serde(default = "default_pyroduct")]
    pub pyroduct: Dependency,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleManifest<Metadata = Value> {
    pub module: Option<Package<Metadata>>,
    pub workspace: Option<Workspace<Metadata>>,
    #[serde(default = "default_pyroduct")]
    pub pyroduct: Dependency,
    #[serde(default)]
    pub capabilities: DepsSet,
    #[serde(default)]
    pub dependencies: DepsSet,
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

fn default_pyroduct() -> Dependency {
    Dependency::Inherited(InheritedDependencyDetail { workspace: true, ..Default::default() })
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

impl CapabilityManifest {
    /// Reads from a file string, processes logic, and returns a standard Manifest 
    /// ready for serialization.
    pub fn to_capability_manifest(self) -> Manifest {
        let mut final_deps = BTreeMap::new();
        final_deps.insert("pyroduct".to_string(), self.pyroduct.clone());
        final_deps.extend(self.dependencies.shared.clone().into_iter());
        self.augment_deps(&mut final_deps, &self.dependencies.host, true);
        self.augment_deps(&mut final_deps, &self.dependencies.module, true);
        let final_features = self.create_requisite_features(&self.features);

        #[allow(deprecated)]
        Manifest {
            package: self.capability.map(ensure_edition_2024),
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

    pub fn to_module_manifest(self) -> Manifest {
        let mut final_deps = BTreeMap::new();

        let pyroduct = match self.pyroduct.clone() {
            // 1) Simple -> Detailed with registry + optional flag
            p @ (Dependency::Simple(_) | Dependency::Inherited(_)) => {p},

            // 2) Detailed -> Add registry only if NOT path or git
            Dependency::Detailed(detail) => {
                let mut d = detail.clone();
                
                if let Some(path) = d.path.as_mut() {
                    *path = format!("../{path}");
                }
                Dependency::Detailed(d)
            },
        };
        
        // 1. Shared Dependencies (Required)
        final_deps.extend(self.dependencies.shared.clone().into_iter());
        final_deps.insert("pyroduct".to_string(), pyroduct);

        // 2. Module Dependencies (Required, NOT optional)
        self.augment_deps(&mut final_deps, &self.dependencies.module, false);

        // 3. Pyroduct

        let mut package = self.capability.clone();
        if let Some(pkg) = &mut package {
            pkg.name = format!("{}-module", pkg.name);
        }

        let final_features = self.features.clone();

        #[allow(deprecated)]
        Manifest {
            package: package.map(ensure_edition_2024),
            workspace: None,
            dependencies: final_deps,
            dev_dependencies: self.dev_dependencies,
            build_dependencies: self.build_dependencies,
            target: self.target,
            features: final_features,
            patch: self.patch,
            lib: self.lib,
            profile: self.profile,
            badges: self.badges,
            bin: Vec::new(),
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

impl ModuleManifest {
    pub fn to_cargo(self) -> Manifest {
        let mut final_deps = BTreeMap::new();
        final_deps.insert("pyroduct".to_string(), self.pyroduct.clone());
        final_deps.extend(self.dependencies.clone().into_iter());
        self.augment_deps(&mut final_deps, &self.capabilities, true);
        
        #[allow(deprecated)]
        Manifest {
            package: self.module.map(ensure_edition_2024),
            workspace: self.workspace,
            dependencies: final_deps,
            dev_dependencies: self.dev_dependencies,
            build_dependencies: self.build_dependencies,
            target: self.target,
            features: BTreeMap::default(),
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

    fn augment_deps(
        &self, 
        target_map: &mut DepsSet, 
        source_map: &DepsSet, 
        make_optional: bool
    ) {
        let registry_url = Some("sparse+http://pyroduct.io/capabilities".to_string());

        for (name, dep) in source_map {
            let new_dep = match dep {
                // 1) Simple -> Detailed with registry + optional flag
                Dependency::Simple(ver) => Dependency::Detailed(Box::new(DependencyDetail {
                    version: Some(ver.clone()),
                    optional: make_optional,
                    registry: registry_url.clone(),
                    ..Default::default()
                })),

                // 2) Detailed -> Add registry only if NOT path or git
                Dependency::Detailed(detail) => {
                    let mut d = detail.clone();
                    
                    // Logic: If it's not a local path and not a git repo, apply the registry
                    if d.path.is_none() && d.git.is_none() && d.registry.is_none() {
                        d.registry = registry_url.clone();
                    }

                    if let Some(path) = d.path.as_mut() {
                        *path = format!("{path}/module");
                    }
                    
                    d.optional = make_optional;
                    Dependency::Detailed(d)
                },

                // 3) Inherited -> Pass through (optional flag still applied if needed)
                Dependency::Inherited(inherited) => {
                    let mut d = inherited.clone();
                    d.optional = make_optional;
                    Dependency::Inherited(d)
                }
            };
            target_map.insert(name.clone(), new_dep);
        }
    }
}

fn ensure_edition_2024<Metadata>(mut package: Package<Metadata>) -> Package<Metadata> {
    if let Inheritable::Inherited = package.edition {
        package.edition = Inheritable::Set(Edition::E2024);
    }
    package
}

#[cfg(test)]
mod tests {
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

        // 2. Convert to Standard Manifest (Augment logic runs here)
        let standard_manifest = cap_manifest.to_capability_manifest();

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