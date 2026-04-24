use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

use crate::core::{
    central_repo, content_hash, error::AppError, installer, skill_store::SkillStore, sync_engine,
    tool_adapters,
};

#[derive(Debug, Serialize)]
pub struct SkillLinkIssue {
    pub tool_key: String,
    pub tool_display_name: String,
    pub skill_name: String,
    pub found_path: String,
    pub issue_type: String,
    pub central_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FixAllResult {
    pub relinked: usize,
    pub deleted: usize,
    pub skipped: usize,
}

fn is_symlink_to_central(path: &Path) -> bool {
    if let Ok(target) = std::fs::read_link(path) {
        let central = central_repo::skills_dir();
        return target.starts_with(&central);
    }
    false
}

fn diagnose_adapter_skills(
    adapter: &tool_adapters::ToolAdapter,
) -> Vec<SkillLinkIssue> {
    let skills_dir = adapter.skills_dir();
    if !skills_dir.exists() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(&skills_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let central = central_repo::skills_dir();
    let mut issues = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip hidden entries
        if name.starts_with('.') {
            continue;
        }

        let is_symlink = path.is_symlink();

        if is_symlink {
            if is_symlink_to_central(&path) {
                // Healthy symlink pointing to central repo — skip
                // But check if the target actually exists (dangling within central)
                if !path.exists() {
                    issues.push(SkillLinkIssue {
                        tool_key: adapter.key.clone(),
                        tool_display_name: adapter.display_name.clone(),
                        skill_name: name,
                        found_path: path.to_string_lossy().to_string(),
                        issue_type: "broken_symlink".to_string(),
                        central_path: None,
                    });
                }
                continue;
            }

            // Symlink but not to central
            if path.exists() {
                // Points somewhere else that exists
                issues.push(SkillLinkIssue {
                    tool_key: adapter.key.clone(),
                    tool_display_name: adapter.display_name.clone(),
                    skill_name: name.clone(),
                    found_path: path.to_string_lossy().to_string(),
                    issue_type: "wrong_target_symlink".to_string(),
                    central_path: if central.join(&name).exists() {
                        Some(central.join(&name).to_string_lossy().to_string())
                    } else {
                        None
                    },
                });
            } else {
                // Broken symlink
                issues.push(SkillLinkIssue {
                    tool_key: adapter.key.clone(),
                    tool_display_name: adapter.display_name.clone(),
                    skill_name: name,
                    found_path: path.to_string_lossy().to_string(),
                    issue_type: "broken_symlink".to_string(),
                    central_path: None,
                });
            }
        } else if path.is_dir() {
            let central_skill = central.join(&name);
            if central_skill.exists() {
                // Physical copy with central counterpart — check divergence
                let local_hash = content_hash::hash_directory(&path).ok();
                let central_hash = content_hash::hash_directory(&central_skill).ok();

                let identical = match (&local_hash, &central_hash) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                };

                issues.push(SkillLinkIssue {
                    tool_key: adapter.key.clone(),
                    tool_display_name: adapter.display_name.clone(),
                    skill_name: name.clone(),
                    found_path: path.to_string_lossy().to_string(),
                    issue_type: if identical {
                        "physical_copy_identical".to_string()
                    } else {
                        "physical_copy_diverged".to_string()
                    },
                    central_path: Some(central_skill.to_string_lossy().to_string()),
                });
            } else {
                // Orphan — not in central
                issues.push(SkillLinkIssue {
                    tool_key: adapter.key.clone(),
                    tool_display_name: adapter.display_name.clone(),
                    skill_name: name,
                    found_path: path.to_string_lossy().to_string(),
                    issue_type: "orphan".to_string(),
                    central_path: None,
                });
            }
        }
    }

    issues
}

#[tauri::command]
pub async fn diagnose_skill_links(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SkillLinkIssue>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = tool_adapters::enabled_installed_adapters(&store);
        let mut all_issues = Vec::new();

        for adapter in &adapters {
            all_issues.extend(diagnose_adapter_skills(adapter));
        }

        Ok(all_issues)
    })
    .await?
}

fn relink_to_central(found_path: &Path, central_path: &Path) -> Result<(), AppError> {
    sync_engine::remove_target(found_path).map_err(AppError::io)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(central_path, found_path).map_err(|e| {
            AppError::io(anyhow::anyhow!(
                "Failed to create symlink {:?} -> {:?}: {}",
                found_path,
                central_path,
                e
            ))
        })?;
    }
    #[cfg(not(unix))]
    {
        // Fallback: copy on non-unix
        sync_engine::sync_skill(central_path, found_path, sync_engine::SyncMode::Copy)
            .map_err(AppError::io)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn fix_skill_link(
    tool_key: String,
    skill_name: String,
    action: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let adapter = tool_adapters::find_adapter_with_store(&store, &tool_key)
            .ok_or_else(|| AppError::not_found(format!("Unknown tool: {tool_key}")))?;

        let found_path = adapter.skills_dir().join(&skill_name);
        let central = central_repo::skills_dir();
        let central_path = central.join(&skill_name);

        match action.as_str() {
            "relink" => {
                if !central_path.exists() {
                    return Err(AppError::not_found(
                        "Central skill does not exist; cannot relink",
                    ));
                }
                relink_to_central(&found_path, &central_path)?;
                Ok(())
            }
            "delete" => {
                sync_engine::remove_target(&found_path).map_err(AppError::io)?;
                Ok(())
            }
            "import_and_relink" => {
                // Import into central repo first
                if !central_path.exists() {
                    installer::install_from_local_to_destination(
                        &found_path,
                        Some(&skill_name),
                        &central_path,
                    )
                    .map_err(AppError::io)?;

                    // Also register in the DB if not already known
                    let central_str = central_path.to_string_lossy().to_string();
                    if store.get_skill_by_central_path(&central_str).map_err(AppError::db)?.is_none() {
                        let meta = crate::core::skill_metadata::parse_skill_md(&central_path);
                        let now = chrono::Utc::now().timestamp_millis();
                        let id = uuid::Uuid::new_v4().to_string();
                        let hash = content_hash::hash_directory(&central_path)
                            .unwrap_or_default();
                        let record = crate::core::skill_store::SkillRecord {
                            id: id.clone(),
                            name: skill_name.clone(),
                            description: meta.description,
                            source_type: "import".to_string(),
                            source_ref: Some(found_path.to_string_lossy().to_string()),
                            source_ref_resolved: None,
                            source_subpath: None,
                            source_branch: None,
                            source_revision: None,
                            remote_revision: None,
                            central_path: central_str,
                            content_hash: Some(hash),
                            enabled: true,
                            created_at: now,
                            updated_at: now,
                            status: "ok".to_string(),
                            update_status: "local_only".to_string(),
                            last_checked_at: Some(now),
                            last_check_error: None,
                        };
                        store.insert_skill(&record).map_err(AppError::db)?;

                        if let Ok(Some(scenario_id)) = store.get_active_scenario_id() {
                            store.add_skill_to_scenario(&scenario_id, &id).ok();
                        }
                    }
                }

                relink_to_central(&found_path, &central_path)?;
                Ok(())
            }
            _ => Err(AppError::invalid_input(format!(
                "Unknown action: {action}"
            ))),
        }
    })
    .await?
}

#[tauri::command]
pub async fn fix_all_skill_links(
    store: State<'_, Arc<SkillStore>>,
) -> Result<FixAllResult, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let adapters = tool_adapters::enabled_installed_adapters(&store);
        let mut all_issues = Vec::new();
        for adapter in &adapters {
            all_issues.extend(diagnose_adapter_skills(adapter));
        }

        let central = central_repo::skills_dir();
        let mut relinked = 0usize;
        let mut deleted = 0usize;
        let mut skipped = 0usize;

        for issue in &all_issues {
            let found = PathBuf::from(&issue.found_path);
            match issue.issue_type.as_str() {
                "physical_copy_identical" => {
                    let central_path = central.join(&issue.skill_name);
                    if central_path.exists() {
                        match relink_to_central(&found, &central_path) {
                            Ok(()) => relinked += 1,
                            Err(e) => {
                                log::warn!(
                                    "Failed to relink {} in {}: {e}",
                                    issue.skill_name,
                                    issue.tool_key
                                );
                                skipped += 1;
                            }
                        }
                    } else {
                        skipped += 1;
                    }
                }
                "broken_symlink" => match sync_engine::remove_target(&found) {
                    Ok(()) => deleted += 1,
                    Err(e) => {
                        log::warn!(
                            "Failed to remove broken symlink {}: {e}",
                            found.display()
                        );
                        skipped += 1;
                    }
                },
                _ => {
                    // physical_copy_diverged, orphan, wrong_target_symlink — need user decision
                    skipped += 1;
                }
            }
        }

        Ok(FixAllResult {
            relinked,
            deleted,
            skipped,
        })
    })
    .await?
}
