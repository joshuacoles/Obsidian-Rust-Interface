use crate::Error::{MissingMetadata, UnclosedMetadata};
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use utils::{is_hidden, is_markdown};
use walkdir::WalkDir;

pub mod joining;
mod utils;

type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error {0:?}")]
    IO(#[from] std::io::Error),

    #[error("No metadata found")]
    MissingMetadata,

    #[error("No closing --- for metadata found")]
    UnclosedMetadata,

    #[error("Error parsing yaml metadata {0:?}")]
    MetadataError(#[from] serde_yaml::Error),

    #[error("Vault was malformed: {0}")]
    MalformedVault(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoteReference {
    path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VaultNote<T> {
    path: PathBuf,
    pub metadata: T,
    pub content: String,
}

impl<T: Serialize> VaultNote<T> {
    pub fn write(&self) -> Result<()> {
        let contents = format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&self.metadata)?,
            self.content
        );

        std::fs::write(&self.path, contents)?;
        Ok(())
    }
}

impl<T> VaultNote<T> {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl NoteReference {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn to_path_buf(self) -> PathBuf {
        self.path
    }

    pub fn parts<T: DeserializeOwned>(&self) -> Result<(Option<T>, String)> {
        let content = std::fs::read_to_string(&self.path)?;
        let mut lines = content.lines();

        let Some(first_line) = lines.next() else {
            return Ok((None, "".to_string()));
        };

        if first_line != "---" {
            return Ok((None, content));
        }

        let metadata_block = lines.take_while_ref(|line| *line != "---").join("\n");

        let metadata = serde_yaml::from_str::<T>(&metadata_block)?;

        // Read next "---" which is left by the take while
        lines.next().ok_or(UnclosedMetadata)?;

        let rest = lines.join("\n");

        Ok((Some(metadata), rest))
    }

    pub fn raw_content(&self) -> Result<String> {
        Ok(std::fs::read_to_string(&self.path)?)
    }

    pub fn metadata<T: DeserializeOwned>(&self) -> Result<T> {
        self.parts()?.0.ok_or(MissingMetadata)
    }

    pub fn from_path(path: &Path) -> NoteReference {
        NoteReference {
            path: path.to_path_buf(),
        }
    }

    pub fn parse<T: DeserializeOwned>(&self) -> Result<VaultNote<T>> {
        let (metadata, content) = self.parts()?;
        let metadata = metadata.ok_or(MissingMetadata)?;
        Ok(VaultNote {
            path: self.path.clone(),
            metadata,
            content,
        })
    }
}

pub struct Vault {
    root: PathBuf,
}

impl Vault {
    pub fn open(root: &Path) -> Vault {
        Vault {
            root: root.to_path_buf(),
        }
    }

    pub fn notes(&self) -> impl Iterator<Item = Result<NoteReference>> {
        let walker = WalkDir::new(&self.root).into_iter();
        walker
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
            .filter(|e| !is_hidden(e) && is_markdown(e))
            .map(|entry| {
                let path = entry.path().to_path_buf();
                Ok(NoteReference { path })
            })
    }
}
