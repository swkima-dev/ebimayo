use std::collections::HashSet;

use crate::skills::model::SkillMetaData;

pub fn merge_unique_skill_metadata(
    subordinated_skills: Vec<SkillMetaData>,
    prioritized_skills: Vec<SkillMetaData>,
) -> Vec<SkillMetaData> {
    let mut seen: HashSet<String> = prioritized_skills.iter().map(|x| x.name.clone()).collect();

    let mut result = prioritized_skills;
    result.extend(
        subordinated_skills
            .into_iter()
            .filter(|x| seen.insert(x.name.clone())),
    );
    result
}
