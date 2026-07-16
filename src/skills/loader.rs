use serde::Deserialize;
use std::{collections::HashSet, ffi::OsStr, fs, path::PathBuf};

use walkdir::WalkDir;

use crate::skills::model::{SkillError, SkillMetaData, SkillWarning};

#[derive(Debug, Deserialize)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
}

fn extract_frontmatter(contents: &str) -> Option<String> {
    let mut lines = contents.lines();
    if !matches!(lines.next(), Some(line) if line.trim() == "---") {
        return None;
    }

    let mut frontmatter_lines: Vec<&str> = Vec::new();
    let mut found_closing = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            found_closing = true;
            break;
        }
        frontmatter_lines.push(line);
    }

    if frontmatter_lines.is_empty() || !found_closing {
        return None;
    }

    Some(frontmatter_lines.join("\n"))
}

pub fn parse_skill_metadata(
    skill_md_path: PathBuf,
) -> Result<(SkillMetaData, Option<SkillWarning>), SkillError> {
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(content) => content,
        Err(e) => return Err(SkillError::IoError(e)),
    };
    let frontmatter = extract_frontmatter(&content).ok_or(SkillError::MissingFrontmatter)?;

    let parsed: SkillFrontmatter = match yaml_serde::from_str(&frontmatter) {
        Ok(parsed) => parsed,
        Err(_) => return Err(SkillError::InvalidYaml),
    };
    let name = parsed.name.ok_or(SkillError::EmptyName)?;
    if name.trim().is_empty() {
        return Err(SkillError::EmptyName);
    }
    let description = parsed.description.ok_or(SkillError::EmptyDescription)?;
    if description.trim().is_empty() {
        return Err(SkillError::EmptyDescription);
    }

    let mut warn: Option<SkillWarning> = None;
    if name.chars().count() > 64 {
        warn = Some(SkillWarning::TooLongName);
    } else if OsStr::new(name.trim()) != skill_md_path.parent().unwrap().file_name().unwrap() {
        warn = Some(SkillWarning::MismatchedDirectoryName);
    }

    Ok((
        SkillMetaData {
            name,
            description,
            path_to_skills_md: skill_md_path,
        },
        warn,
    ))
}

pub fn find_skills_path(target: PathBuf) -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(target).max_depth(2) {
        if let Ok(element) = entry {
            if element.file_type().is_file()
                && element.clone().into_path().file_name() == Some(OsStr::new("SKILL.md"))
            {
                result.push(element.into_path());
            }
        }
    }
    result
}

pub fn load_skills(target_dir: PathBuf) -> Vec<SkillMetaData> {
    let mut loaded_skills: Vec<SkillMetaData> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for skill_md_path in find_skills_path(target_dir) {
        let parsed_metadata = parse_skill_metadata(skill_md_path);
        let metadata = match parsed_metadata {
            Ok((metadata, _)) => metadata, // In the future, the warning content should be displayed to the user.
            Err(_) => {
                continue;
                // In the future, error details should be logged.
            }
        };

        if seen.insert(metadata.name.clone()) {
            loaded_skills.push(metadata);
        } else if let Some(pos) = loaded_skills.iter().position(|x| x.name == metadata.name) {
            loaded_skills[pos] = metadata;
        }
    }

    loaded_skills
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::skills::{
        loader::{extract_frontmatter, find_skills_path, load_skills, parse_skill_metadata},
        model::{SkillError, SkillMetaData, SkillWarning},
    };

    #[test]
    fn find_skills() {
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tests/inputs/agent_skills");

        let expected = vec![
            base.join("my-skill-a/SKILL.md"),
            base.join("my-skill-b/SKILL.md"),
        ];

        let mut result = find_skills_path(base);
        result.sort();

        assert_eq!(expected, result);
    }

    #[test]
    fn success_extract_frontmatter() {
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tests/inputs/agent_skills/my-skill-a/SKILL.md");

        let raw = fs::read_to_string(base).unwrap();

        let expected = "name: my-skill-a
description: A agent skill for test.";

        assert_eq!(expected, extract_frontmatter(&raw).unwrap());
    }

    #[test]
    fn success_parse_skill_metadata() {
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tests/inputs/agent_skills/my-skill-a/SKILL.md");

        let expected = (
            SkillMetaData {
                name: "my-skill-a".to_string(),
                description: "A agent skill for test.".to_string(),
                path_to_skills_md: base.clone(),
            },
            None::<SkillWarning>,
        );

        assert_eq!(expected, parse_skill_metadata(base).unwrap());
    }

    #[test]
    fn empty_description_parse_skill_metadata() {
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tests/inputs/agent_skills/my-skill-b/SKILL.md");

        let result = parse_skill_metadata(base);

        assert!(matches!(result, Err(SkillError::EmptyDescription)));
    }

    #[test]
    fn success_loaded_skills() {
        let mut base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.push("tests/inputs/agent_skills");
        let mut path_to_my_skill_a = base.clone();
        path_to_my_skill_a.push("my-skill-a/SKILL.md");

        let expected = vec![SkillMetaData {
            name: "my-skill-a".to_string(),
            description: "A agent skill for test.".to_string(),
            path_to_skills_md: path_to_my_skill_a,
        }];

        assert_eq!(expected, load_skills(base));
    }
}
