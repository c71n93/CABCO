use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use globset::{GlobBuilder, GlobMatcher};
use serde::Deserialize;
use serde::de::{MapAccess, Visitor};
use thiserror::Error;

use crate::core::{CollectedSnapshot, Ownership, PathDetails};

#[derive(Debug, Error)]
pub enum DeclarationError {
    #[error("snapshot {snapshot} has invalid YAML: {message}")]
    InvalidYaml { snapshot: String, message: String },
    #[error("snapshot {snapshot} uses unsupported version {version}; expected 1")]
    UnsupportedVersion { snapshot: String, version: i64 },
    #[error(
        "snapshot {snapshot} component {component:?} has invalid {field} pattern {pattern:?}: {message}"
    )]
    InvalidPattern {
        snapshot: String,
        component: String,
        field: &'static str,
        pattern: String,
        message: String,
    },
    #[error("snapshot {snapshot} file {path:?} belongs to conflicting components: {components}")]
    ConflictingComponents {
        snapshot: String,
        path: String,
        components: String,
    },
    #[error(
        "snapshot {snapshot} file {path:?} is an interface boundary of {component:?} but does not belong to that component"
    )]
    ExternalBoundary {
        snapshot: String,
        path: String,
        component: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: i64,
    components: Components,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComponentDocument {
    ownership: Ownership,
    files: Vec<String>,
    interfaces: Vec<String>,
}

#[derive(Debug)]
struct Components(BTreeMap<String, ComponentDocument>);

impl<'de> Deserialize<'de> for Components {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ComponentsVisitor;

        impl<'de> Visitor<'de> for ComponentsVisitor {
            type Value = Components;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a mapping of component names to declarations")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut components = BTreeMap::new();
                while let Some((name, component)) = map.next_entry::<String, ComponentDocument>()? {
                    if components.insert(name.clone(), component).is_some() {
                        return Err(serde::de::Error::custom(format!("duplicate key {name:?}")));
                    }
                }
                Ok(Components(components))
            }
        }

        deserializer.deserialize_map(ComponentsVisitor)
    }
}

struct Component {
    name: String,
    ownership: Ownership,
    files: Vec<GlobMatcher>,
    interfaces: Vec<GlobMatcher>,
}

pub(crate) struct ResolvedDeclaration {
    pub membership: BTreeMap<String, PathDetails>,
}

impl ResolvedDeclaration {
    pub fn details(&self, path: &str) -> PathDetails {
        self.membership.get(path).cloned().unwrap_or(PathDetails {
            component: None,
            ownership: None,
            interface_boundary: false,
        })
    }
}

fn compile_patterns(
    patterns: Vec<String>,
    snapshot: &str,
    component: &str,
    field: &'static str,
) -> Result<Vec<GlobMatcher>, DeclarationError> {
    patterns
        .into_iter()
        .map(|pattern| {
            GlobBuilder::new(&pattern)
                .literal_separator(true)
                .case_insensitive(false)
                .build()
                .map(|glob| glob.compile_matcher())
                .map_err(|error| DeclarationError::InvalidPattern {
                    snapshot: snapshot.into(),
                    component: component.into(),
                    field,
                    pattern,
                    message: error.to_string(),
                })
        })
        .collect()
}

pub(crate) fn resolve(
    snapshot: &CollectedSnapshot,
) -> Result<ResolvedDeclaration, DeclarationError> {
    let Some(content) = &snapshot.declaration else {
        return Ok(ResolvedDeclaration {
            membership: BTreeMap::new(),
        });
    };
    let document: Document =
        serde_yaml::from_slice(content).map_err(|error| DeclarationError::InvalidYaml {
            snapshot: snapshot.commit.clone(),
            message: error.to_string(),
        })?;
    if document.version != 1 {
        return Err(DeclarationError::UnsupportedVersion {
            snapshot: snapshot.commit.clone(),
            version: document.version,
        });
    }

    let mut components = Vec::with_capacity(document.components.0.len());
    for (name, component) in document.components.0 {
        components.push(Component {
            files: compile_patterns(component.files, &snapshot.commit, &name, "files")?,
            interfaces: compile_patterns(
                component.interfaces,
                &snapshot.commit,
                &name,
                "interfaces",
            )?,
            name,
            ownership: component.ownership,
        });
    }

    let mut membership = BTreeMap::new();
    for path in &snapshot.files {
        let owners: Vec<&Component> = components
            .iter()
            .filter(|component| component.files.iter().any(|pattern| pattern.is_match(path)))
            .collect();
        if owners.len() > 1 {
            return Err(DeclarationError::ConflictingComponents {
                snapshot: snapshot.commit.clone(),
                path: path.clone(),
                components: owners
                    .iter()
                    .map(|component| component.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
        if let Some(component) = owners.first() {
            membership.insert(
                path.clone(),
                PathDetails {
                    component: Some(component.name.clone()),
                    ownership: Some(component.ownership),
                    interface_boundary: false,
                },
            );
        }
    }

    let mut boundaries = BTreeSet::new();
    for component in &components {
        for path in &snapshot.files {
            if component
                .interfaces
                .iter()
                .any(|pattern| pattern.is_match(path))
            {
                if membership
                    .get(path)
                    .and_then(|details| details.component.as_deref())
                    != Some(component.name.as_str())
                {
                    return Err(DeclarationError::ExternalBoundary {
                        snapshot: snapshot.commit.clone(),
                        path: path.clone(),
                        component: component.name.clone(),
                    });
                }
                boundaries.insert(path.clone());
            }
        }
    }
    for path in boundaries {
        membership
            .get_mut(&path)
            .expect("boundary membership checked")
            .interface_boundary = true;
    }

    Ok(ResolvedDeclaration { membership })
}
