use std::{collections::HashMap, path::PathBuf};

use crate::diagnostic::FileId;

#[derive(Debug, Default)]
pub struct Files {
    files: HashMap<FileId, File>,
}

#[derive(Clone, Debug)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
}

impl Files {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add(&mut self, file: File) -> FileId {
        let id = FileId {
            index: self.files.len() as u32,
        };

        self.files.insert(id, file);

        id
    }

    pub fn get(&self, file: FileId) -> Option<&File> {
        self.files.get(&file)
    }
}
