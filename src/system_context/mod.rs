use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::skills::{loader::load_skills, model::SkillMetaData, util::merge_unique_skill_metadata};

pub struct SystemContexts {
    system_prompt: String,
    workspace_dir: PathBuf,
    instruction: Option<PathBuf>,
    os: String,
    skills: Vec<SkillMetaData>,
}

impl SystemContexts {
    pub fn new() -> Self {
        let system_prompt = include_str!("system.txt").to_string();
        let workspace_dir =
            env::current_dir().expect("current directory should always be accessible at startup");
        let instruction = Self::find_instruction_file(&workspace_dir);
        let os = std::env::consts::OS.to_string();
        let skills = Vec::new();

        Self {
            system_prompt,
            workspace_dir,
            instruction,
            os,
            skills,
        }
    }

    pub fn reload_skills(&mut self) -> &mut Self {
        let user_level_dir = std::env::home_dir();
        let mut skills_result: Vec<SkillMetaData> = Vec::new();
        let mut base_dir: Vec<PathBuf> = vec![self.workspace_dir.clone()];

        if user_level_dir.is_some() {
            base_dir.push(user_level_dir.unwrap());
        }

        for dir in base_dir {
            let mut ebimayo_dir = dir.clone();
            ebimayo_dir.push(".ebimayo/skills/");
            let mut agent_dir = dir;
            agent_dir.push(".agents/skills/");

            let ebimayo_dir_skills = load_skills(ebimayo_dir);
            let agent_dir_skills = load_skills(agent_dir);

            skills_result = merge_unique_skill_metadata(ebimayo_dir_skills, skills_result);
            skills_result = merge_unique_skill_metadata(agent_dir_skills, skills_result);
        }

        self.skills = skills_result;
        self
    }

    fn build_skill_catalog(&self) -> String {
        let mut catalog = "<available_skills>\n".to_string();

        for skill in self.skills.iter() {
            catalog.push_str(&format!("  <skill>    <name>{}</name>\n    <description>{}</description>\n    <location>{}</location>\n  </skill>\n",
                    skill.name, skill.description, skill.path_to_skills_md.to_str().unwrap()));
        }
        catalog.push_str("</available_skills>");
        catalog
    }

    pub fn prompt(&self) -> String {
        let mut prompt = String::new();

        // default system prompt
        prompt.push_str(&self.system_prompt);
        prompt.push_str("\n\n");

        // instruction file
        prompt.push_str(
            &self
                .instruction
                .as_ref()
                .and_then(|path| fs::read_to_string(path).ok())
                .unwrap_or_default(),
        );
        prompt.push_str("\n\n");

        // environment information
        prompt.push_str(&format!(
            "<env>Workspace directory: {:?}\nOS: {}</env>",
            self.workspace_dir, self.os,
        ));
        prompt.push_str("\n\n");

        if !self.skills.is_empty() {
            prompt.push_str(&self.build_skill_catalog());
            prompt.push_str(include_str!("agent_skills.txt"));
        }
        prompt
    }

    fn find_instruction_file(root: &Path) -> Option<PathBuf> {
        let candidates = ["AGENTS.md", "EBIMAYO.md", "CONTEXT.md"];

        candidates
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.exists())
    }
}

impl Default for SystemContexts {
    fn default() -> Self {
        Self::new()
    }
}
