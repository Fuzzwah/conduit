use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillReference {
    pub name: String,
    pub path: PathBuf,
}
