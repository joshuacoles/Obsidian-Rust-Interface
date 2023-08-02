use std::collections::HashMap;
use std::hash::Hash;
use crate::Error::MalformedVault;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::joining::WriteOutcome::*;
use crate::{NoteReference, Vault};
use crate::joining::strategies::Strategy;

pub mod strategies {
    use serde::de::DeserializeOwned;
    use serde_yaml::from_value;
    use std::hash::Hash;

    use crate::joining::WriteOutcome::*;
    use crate::NoteReference;

    pub trait Strategy<K> {
        fn extract(&self, note_reference: NoteReference) -> Option<(K, NoteReference)>;
    }

    pub struct Branded {
        brand_key: String,
    }

    impl<K: DeserializeOwned> Strategy<K> for Branded {
        fn extract(&self, note_reference: NoteReference) -> Option<(K, NoteReference)> {
            let yaml = note_reference.metadata::<serde_yaml::Mapping>().ok()?;
            let brand = yaml.get(&self.brand_key)?;
            let brand: K = from_value(brand.clone()).ok()?;
            Some((brand, note_reference))
        }
    }

    pub struct TypeAndKey {
        type_key: String,
        note_type: String,

        id_key: String,
    }

    impl<K: DeserializeOwned> Strategy<K> for TypeAndKey {
        fn extract(&self, note_reference: NoteReference) -> Option<(K, NoteReference)> {
            let yaml = note_reference.metadata::<serde_yaml::Mapping>().ok()?;
            let note_type = yaml.get(&self.type_key)?.as_str()?;

            if note_type != self.note_type {
                return None;
            }

            let id = yaml.get(&self.id_key)?;
            let id: K = from_value(id.clone()).ok()?;
            Some((id, note_reference))
        }
    }
}

pub fn find_by<S: Strategy<K>, K>(vault: &Vault, strategy: &S) -> HashMap<K, NoteReference>
    where
        K: Eq + Hash,
{
    vault
        .notes()
        .filter_map(|n| n.ok())
        .filter_map(|n| strategy.extract(n))
        .collect()
}

/// A joined note is a note that corresponds with some resource outside of Obsidian.
/// It has a default path, as well as a brand and id used to locate the object if it exists in the
/// file system already.
pub struct JoinedNote<K, T> {
    pub note_id: K,

    pub default_path: PathBuf,
    pub metadata: T,
    pub contents: String,
}

#[derive(Clone, Copy, Debug)]
pub enum WriteOutcome {
    Created,
    Updated,
}

impl<K, T: Serialize> JoinedNote<K, T> {
    pub fn write(&self, existing: Option<&PathBuf>) -> Result<WriteOutcome, crate::Error> {
        let (outcome, path) = if let Some(existing) = existing {
            (Updated, existing)
        } else {
            let parent = self
                .default_path
                .parent()
                .filter(|p| *p != Path::new(""))
                .ok_or(MalformedVault(
                    "Invalid note location, lacks meaningful parent".to_string(),
                ))?;

            std::fs::create_dir_all(parent)?;
            (Created, &self.default_path)
        };

        debug!("Writing note to {:?}", &path);

        let contents = format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&self.metadata)?,
            self.contents
        );

        std::fs::write(&path, contents)?;
        Ok(outcome)
    }
}
