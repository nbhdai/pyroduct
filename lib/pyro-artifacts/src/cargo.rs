//! Conversions between our manifest and Cargo's

pub use cargo_toml::Dependency;
use cargo_toml::{
    Badges, DependencyDetail, DepsSet, Edition, FeatureSet, Inheritable, InheritedDependencyDetail,
    LintGroups, Manifest, Package, PatchSet, Product, Profiles, TargetDepsSet, Workspace,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use toml::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityIdent {
    pub name: String,
    pub version: String,
    pub author: String,
}

impl CapabilityIdent {
    pub fn to_package(self) -> Package<Value> {
        let mut package = Package::new(self.name, self.version);
        package.authors = Inheritable::Set(vec![self.author]);
        package.edition = Inheritable::Set(Edition::E2024);
        package
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectManifest {
    Capability(CapabilityManifest),
    Module(ModuleManifest),
}

impl ProjectManifest {
    pub fn ident(&self) -> &CapabilityIdent {
        match self {
            ProjectManifest::Capability(c) => &c.capability,
            ProjectManifest::Module(m) => &m.module,
        }
    }

    pub fn to_cargo_manifest(self, cache_manager: Option<&crate::cache::CacheManager>) -> Manifest {
        match self {
            ProjectManifest::Capability(c) => c.to_capability_manifest(),
            ProjectManifest::Module(m) => m.to_cargo(cache_manager),
        }
    }

    pub fn to_interface_manifest(self) -> Option<Manifest> {
        match self {
            ProjectManifest::Capability(c) => Some(c.to_interface_manifest()),
            ProjectManifest::Module(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CapabilityManifest<Metadata = Value> {
    pub capability: CapabilityIdent,
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

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("Pyroduct does not support inherited versions (yet!)")]
    InheritedVersionNotSupported,
    #[error("[capability] section is missing")]
    CapabilitySectionMissing,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedCapability {
    pub author: String,
    pub package: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModuleManifest<Metadata = Value> {
    pub module: CapabilityIdent,
    pub workspace: Option<Workspace<Metadata>>,
    #[serde(default = "default_pyroduct")]
    pub pyroduct: Dependency,
    #[serde(default)]
    pub capabilities: BTreeMap<String, ResolvedCapability>,
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
    Dependency::Inherited(InheritedDependencyDetail {
        workspace: true,
        ..Default::default()
    })
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
        let mut pyro_dep = self.pyroduct.clone();
        pyro_dep
            .detail_mut()
            .features
            .push("capability".to_string());
        final_deps.insert("pyroduct".to_string(), pyro_dep);
        final_deps.extend(self.dependencies.shared.clone());
        self.augment_deps(&mut final_deps, &self.dependencies.host, true);
        self.augment_deps(&mut final_deps, &self.dependencies.module, true);
        let final_features = self.create_requisite_features(&self.features);

        #[allow(deprecated)]
        Manifest {
            package: Some(self.capability.to_package()),
            workspace: self.workspace,
            dependencies: final_deps,
            dev_dependencies: self.dev_dependencies,
            build_dependencies: self.build_dependencies,
            target: self.target,
            features: final_features,
            patch: self.patch,
            lib: ensure_cdylib(self.lib),
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

    pub fn to_interface_manifest(self) -> Manifest {
        let mut final_deps = BTreeMap::new();

        // Use a simple version dependency for pyroduct to allow patching by the builder
        let mut pyro_dep = self.pyroduct.clone();
        pyro_dep.detail_mut().features.push("module".to_string());

        // 1. Shared Dependencies (Required)
        final_deps.extend(self.dependencies.shared.clone());
        final_deps.insert("pyroduct".to_string(), pyro_dep);

        // 2. Module Dependencies (Required, NOT optional)
        self.augment_deps(&mut final_deps, &self.dependencies.module, false);

        // 3. Pyroduct

        let final_features = self.features.clone();

        #[allow(deprecated)]
        Manifest {
            package: Some(self.capability.to_package()),
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
    fn augment_deps(&self, target_map: &mut DepsSet, source_map: &DepsSet, make_optional: bool) {
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
                    }
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
        let capability_feature: Vec<String> = self
            .dependencies
            .host
            .keys()
            .map(|name| format!("dep:{}", name))
            .collect();

        let module_feature: Vec<String> = self
            .dependencies
            .module
            .keys()
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
    pub fn to_cargo(self, cache_manager: Option<&crate::cache::CacheManager>) -> Manifest {
        let mut final_deps = BTreeMap::new();
        let mut pyro_dep = self.pyroduct.clone();
        pyro_dep.detail_mut().features.push("module".to_string());
        final_deps.insert("pyroduct".to_string(), pyro_dep);
        final_deps.extend(self.dependencies.clone());
        self.augment_deps(&mut final_deps, &self.capabilities, cache_manager);

        #[allow(deprecated)]
        Manifest {
            package: Some(self.module.to_package()),
            workspace: self.workspace,
            dependencies: final_deps,
            dev_dependencies: self.dev_dependencies,
            build_dependencies: self.build_dependencies,
            target: self.target,
            features: BTreeMap::default(),
            patch: self.patch,
            lib: ensure_cdylib(self.lib),
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
        capabilities: &BTreeMap<String, ResolvedCapability>,
        cache_manager: Option<&crate::cache::CacheManager>,
    ) {
        for (name, cap) in capabilities.iter() {
            let path = if let Some(cm) = cache_manager {
                cm.interface_dir(&cap.author, &cap.package, &cap.version)
                    .to_string_lossy()
                    .into()
            } else {
                Path::new("..")
                    .join(&cap.author)
                    .join(&cap.package)
                    .join(&cap.version)
                    .to_string_lossy()
                    .into()
            };
            let dep = Dependency::Detailed(Box::new(DependencyDetail {
                path: Some(path),
                ..Default::default()
            }));
            target_map.insert(name.clone(), dep);
        }
    }
}

pub fn ensure_cdylib(lib: Option<Product>) -> Option<Product> {
    let lib = if let Some(mut lib) = lib {
        if !lib.crate_type.iter().any(|s| s.as_str() == "cdylib") {
            lib.crate_type.push("cdylib".to_string());
            lib
        } else {
            lib
        }
    } else {
        Product {
            crate_type: vec!["cdylib".to_string()],
            ..Default::default()
        }
    };
    Some(lib)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_transformation() {
        let input_toml = r#"
[capability]
name = "my-capability"
version = "0.1.0"
author = "Me"

[pyroduct]
path = "../../lib/pyroduct"

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
            Dependency::Detailed(d) => assert!(d.optional),
            _ => panic!("tokio should be detailed"),
        }

        // Check Shared (remains not optional)
        match deps.get("serde").unwrap() {
            Dependency::Detailed(d) => assert!(!d.optional),
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
