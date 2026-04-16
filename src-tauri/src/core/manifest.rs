use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::skill_store::SkillStore;

const MANIFEST_VERSION: u32 = 1;
const MANIFEST_FILENAME: &str = "skills-manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsManifest {
    pub version: u32,
    pub exported_at: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub settings: HashMap<String, String>,
    pub skills: Vec<ManifestSkill>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<ManifestScenario>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_scenario: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSkill {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref_resolved: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_subpath: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tool_targets: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestScenario {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub sort_order: i32,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub skill_tool_toggles: HashMap<String, HashMap<String, bool>>,
}

const SYNCED_SETTINGS_KEYS: &[&str] = &[
    "sync_mode",
    "disabled_tools",
    "custom_tools",
    "custom_tool_paths",
    "default_scenario",
    "update_check_ttl_minutes",
];

pub fn export_manifest(store: &SkillStore, skills_dir: &Path) -> Result<()> {
    let all_skills = store.get_all_skills().context("Failed to read skills")?;
    let all_targets = store.get_all_targets().context("Failed to read targets")?;
    let tags_map = store.get_tags_map().context("Failed to read tags")?;
    let all_scenarios = store
        .get_all_scenarios()
        .context("Failed to read scenarios")?;
    let active_scenario_id = store.get_active_scenario_id().ok().flatten();

    let scenario_id_to_name: HashMap<String, String> = all_scenarios
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    let mut settings = HashMap::new();
    for key in SYNCED_SETTINGS_KEYS {
        if let Ok(Some(value)) = store.get_setting(key) {
            if !value.is_empty() {
                settings.insert(key.to_string(), value);
            }
        }
    }

    let manifest_skills: Vec<ManifestSkill> = all_skills
        .iter()
        .map(|skill| {
            let skill_tags = tags_map.get(&skill.id).cloned().unwrap_or_default();
            let skill_scenarios: Vec<String> = store
                .get_scenarios_for_skill(&skill.id)
                .unwrap_or_default()
                .iter()
                .filter_map(|sid| scenario_id_to_name.get(sid).cloned())
                .collect();

            let targets_for_skill: HashMap<String, String> = all_targets
                .iter()
                .filter(|t| t.skill_id == skill.id)
                .map(|t| (t.tool.clone(), t.mode.clone()))
                .collect();

            ManifestSkill {
                name: skill.name.clone(),
                description: skill.description.clone(),
                source_type: skill.source_type.clone(),
                source_ref: skill.source_ref.clone(),
                source_ref_resolved: skill.source_ref_resolved.clone(),
                source_subpath: skill.source_subpath.clone(),
                source_branch: skill.source_branch.clone(),
                source_revision: skill.source_revision.clone(),
                remote_revision: skill.remote_revision.clone(),
                content_hash: skill.content_hash.clone(),
                enabled: skill.enabled,
                tags: skill_tags,
                scenarios: skill_scenarios,
                tool_targets: targets_for_skill,
            }
        })
        .collect();

    let manifest_scenarios: Vec<ManifestScenario> = all_scenarios
        .iter()
        .map(|scenario| {
            let skill_ids = store
                .get_skill_ids_for_scenario(&scenario.id)
                .unwrap_or_default();

            let mut skill_tool_toggles: HashMap<String, HashMap<String, bool>> = HashMap::new();
            for skill_id in &skill_ids {
                let skill_name = all_skills
                    .iter()
                    .find(|s| s.id == *skill_id)
                    .map(|s| s.name.clone());
                if let Some(name) = skill_name {
                    let toggles = store
                        .get_scenario_skill_tool_toggles(&scenario.id, skill_id)
                        .unwrap_or_default();
                    if !toggles.is_empty() {
                        let tool_map: HashMap<String, bool> =
                            toggles.into_iter().map(|t| (t.tool, t.enabled)).collect();
                        skill_tool_toggles.insert(name, tool_map);
                    }
                }
            }

            ManifestScenario {
                name: scenario.name.clone(),
                description: scenario.description.clone(),
                icon: scenario.icon.clone(),
                sort_order: scenario.sort_order,
                skill_tool_toggles,
            }
        })
        .collect();

    let active_scenario_name = active_scenario_id
        .as_deref()
        .and_then(|id| scenario_id_to_name.get(id).cloned());

    let manifest = SkillsManifest {
        version: MANIFEST_VERSION,
        exported_at: chrono::Utc::now().to_rfc3339(),
        settings,
        skills: manifest_skills,
        scenarios: manifest_scenarios,
        active_scenario: active_scenario_name,
    };

    let json = serde_json::to_string_pretty(&manifest).context("Failed to serialize manifest")?;
    let manifest_path = skills_dir.join(MANIFEST_FILENAME);
    std::fs::write(&manifest_path, json)
        .with_context(|| format!("Failed to write manifest to {:?}", manifest_path))?;

    Ok(())
}

pub fn read_manifest(skills_dir: &Path) -> Result<Option<SkillsManifest>> {
    let manifest_path = skills_dir.join(MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(&manifest_path).context("Failed to read manifest file")?;
    let manifest: SkillsManifest =
        serde_json::from_str(&content).context("Failed to parse manifest")?;
    Ok(Some(manifest))
}

pub fn import_manifest(store: &SkillStore, skills_dir: &Path) -> Result<ImportResult> {
    let manifest = match read_manifest(skills_dir)? {
        Some(m) => m,
        None => return Ok(ImportResult::default()),
    };

    if manifest.version > MANIFEST_VERSION {
        anyhow::bail!(
            "Manifest version {} is newer than supported ({})",
            manifest.version,
            MANIFEST_VERSION
        );
    }

    let mut result = ImportResult::default();

    for (key, value) in &manifest.settings {
        if SYNCED_SETTINGS_KEYS.contains(&key.as_str()) {
            store
                .set_setting(key, value)
                .with_context(|| format!("Failed to restore setting: {}", key))?;
        }
    }

    let mut scenario_name_to_id: HashMap<String, String> = HashMap::new();
    for ms in &manifest.scenarios {
        if let Ok(existing) = store.get_all_scenarios() {
            if let Some(s) = existing.iter().find(|s| s.name == ms.name) {
                scenario_name_to_id.insert(ms.name.clone(), s.id.clone());
                continue;
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let record = super::skill_store::ScenarioRecord {
            id: id.clone(),
            name: ms.name.clone(),
            description: ms.description.clone(),
            icon: ms.icon.clone(),
            sort_order: ms.sort_order,
            created_at: now,
            updated_at: now,
        };
        store
            .insert_scenario(&record)
            .with_context(|| format!("Failed to create scenario: {}", ms.name))?;
        scenario_name_to_id.insert(ms.name.clone(), id);
        result.scenarios_created += 1;
    }

    for ms in &manifest.skills {
        let skill_dir = skills_dir.join(&ms.name);
        if !skill_dir.exists() {
            result.skills_missing += 1;
            continue;
        }

        let central_path = skill_dir.to_string_lossy().to_string();
        if store
            .get_skill_by_central_path(&central_path)
            .ok()
            .flatten()
            .is_some()
        {
            result.skills_skipped += 1;
            continue;
        }

        let now = chrono::Utc::now().timestamp_millis();
        let content_hash = super::content_hash::hash_directory(&skill_dir).ok();
        let id = uuid::Uuid::new_v4().to_string();

        let record = super::skill_store::SkillRecord {
            id: id.clone(),
            name: ms.name.clone(),
            description: ms.description.clone(),
            source_type: ms.source_type.clone(),
            source_ref: ms.source_ref.clone(),
            source_ref_resolved: ms.source_ref_resolved.clone(),
            source_subpath: ms.source_subpath.clone(),
            source_branch: ms.source_branch.clone(),
            source_revision: ms.source_revision.clone(),
            remote_revision: ms.remote_revision.clone(),
            central_path,
            content_hash: ms.content_hash.clone().or(content_hash),
            enabled: ms.enabled,
            created_at: now,
            updated_at: now,
            status: "ok".to_string(),
            update_status: match ms.source_type.as_str() {
                "git" | "skillssh" if ms.source_revision.is_some() => {
                    match (&ms.source_revision, &ms.remote_revision) {
                        (Some(src), Some(rem)) if src == rem => "up_to_date".to_string(),
                        (Some(_), Some(_)) => "update_available".to_string(),
                        _ => "up_to_date".to_string(),
                    }
                }
                "git" | "skillssh" => "unknown".to_string(),
                _ => "local_only".to_string(),
            },
            last_checked_at: None,
            last_check_error: None,
        };
        store
            .insert_skill(&record)
            .with_context(|| format!("Failed to import skill: {}", ms.name))?;

        if !ms.tags.is_empty() {
            store.set_tags_for_skill(&id, &ms.tags).ok();
        }

        for scenario_name in &ms.scenarios {
            if let Some(scenario_id) = scenario_name_to_id.get(scenario_name) {
                store.add_skill_to_scenario(scenario_id, &id).ok();
            }
        }

        result.skills_imported += 1;
    }

    for ms in &manifest.scenarios {
        if let Some(scenario_id) = scenario_name_to_id.get(&ms.name) {
            for (skill_name, tool_toggles) in &ms.skill_tool_toggles {
                let skill = store
                    .get_all_skills()
                    .ok()
                    .and_then(|skills| skills.into_iter().find(|s| s.name == *skill_name));
                if let Some(skill) = skill {
                    for (tool, enabled) in tool_toggles {
                        store
                            .set_scenario_skill_tool_enabled(scenario_id, &skill.id, tool, *enabled)
                            .ok();
                    }
                }
            }
        }
    }

    if let Some(active_name) = &manifest.active_scenario {
        if let Some(active_id) = scenario_name_to_id.get(active_name) {
            store.set_active_scenario(active_id).ok();
        }
    }

    Ok(result)
}

#[derive(Debug, Default)]
pub struct ImportResult {
    pub skills_imported: usize,
    pub skills_skipped: usize,
    pub skills_missing: usize,
    pub scenarios_created: usize,
}
