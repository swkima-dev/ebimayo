use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct SkillMetaData {
    pub name: String,
    pub description: String,
    pub path_to_skills_md: PathBuf,
}

#[derive(Error, Debug, PartialEq)]
pub enum SkillWarning {
    #[error("mismatches directory name and name field")]
    MismatchedDirectoryName,
    #[error("too long name field")]
    TooLongName,
}

#[derive(Error, Debug)]
pub enum SkillError {
    #[error("empty name")]
    EmptyName,
    #[error("empty description")]
    EmptyDescription,
    #[error("invalid yaml")]
    InvalidYaml,
    #[error("io error {0}")]
    IoError(io::Error),
    #[error("missing frontmatter")]
    MissingFrontmatter,
}
