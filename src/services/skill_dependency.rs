//! Skill Dependency Service - Resolves and manages skill dependencies

use crate::db::repositories::session_context::{SessionContextRepository, NewSkillDependency};
use crate::db::repositories::skill::SkillRepository;
use crate::models::error::AppError;
use crate::models::skill::Skill as ModelSkill;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SkillDependency {
    pub skill_id: String,
    pub dependency_skill_id: String,
    pub version_constraint: String,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedSkill {
    pub skill_id: String,
    pub skill: Option<crate::models::skill::Skill>,
    pub state: JsonValue,
}

/// Service for resolving and managing skill dependencies
#[derive(Clone)]
pub struct SkillDependencyService {
    context_repo: SessionContextRepository,
    skill_repo: SkillRepository,
}

impl std::fmt::Debug for SkillDependencyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillDependencyService").finish()
    }
}

impl SkillDependencyService {
    pub fn new(context_repo: SessionContextRepository, skill_repo: SkillRepository) -> Self {
        Self { context_repo, skill_repo }
    }

    pub async fn parse_skill_dependencies(&self, skill: &ModelSkill) -> Result<Vec<SkillDependency>, AppError> {
        let mut dependencies = Vec::new();

        let content = &skill.content;
        if let Some(deps) = self.extract_dependencies_from_content(content) {
            for dep in deps {
                dependencies.push(SkillDependency {
                    skill_id: skill.id.clone(),
                    dependency_skill_id: dep.0,
                    version_constraint: dep.1,
                    is_optional: false,
                });
            }
        }

        for dep_skill_id in &skill.dependencies {
            if !dependencies.iter().any(|d| d.dependency_skill_id == *dep_skill_id) {
                dependencies.push(SkillDependency {
                    skill_id: skill.id.clone(),
                    dependency_skill_id: dep_skill_id.clone(),
                    version_constraint: "*".to_string(),
                    is_optional: false,
                });
            }
        }

        Ok(dependencies)
    }

    fn extract_dependencies_from_content(&self, content: &str) -> Option<Vec<(String, String)>> {
        let mut deps = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("depends_on:") || line.starts_with("dependencies:") {
                let value = line.split(':').nth(1)?.trim();

                for dep in value.split(',') {
                    let dep = dep.trim();
                    if dep.is_empty() {
                        continue;
                    }

                    if dep.starts_with("skill-") {
                        deps.push((dep.to_string(), "*".to_string()));
                    } else if dep.contains('@') {
                        let parts: Vec<&str> = dep.split('@').collect();
                        if parts.len() == 2 {
                            deps.push((parts[0].to_string(), parts[1].to_string()));
                        }
                    } else {
                        deps.push((dep.to_string(), "*".to_string()));
                    }
                }
            }
        }

        if deps.is_empty() {
            None
        } else {
            Some(deps)
        }
    }

    pub async fn register_dependencies(&self, skill_id: &str, dependencies: Vec<SkillDependency>) -> Result<(), AppError> {
        for dep in dependencies {
            let new_dep = NewSkillDependency {
                skill_id: skill_id.to_string(),
                dependency_skill_id: dep.dependency_skill_id,
                version_constraint: dep.version_constraint,
                is_optional: dep.is_optional,
            };
            self.context_repo.add_skill_dependency(new_dep)
                .await
                .map_err(|e| AppError::InternalError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn resolve_dependencies(&self, skill_ids: Vec<String>) -> Result<Vec<ResolvedSkill>, AppError> {
        let resolved_ids = self.context_repo.resolve_dependencies(skill_ids)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut resolved = Vec::new();
        for skill_id_str in resolved_ids {
            let repo_skill = self.skill_repo.find_by_id(&skill_id_str)
                .await
                .map_err(|e| AppError::InternalError(e.to_string()))?;

            let model_skill = repo_skill.map(|s| crate::models::skill::Skill {
                id: s.id,
                name: s.name,
                description: s.description,
                tags: s.tags,
                version: s.version,
                author_agent_id: s.author_agent_id,
                author_identity_id: s.author_identity_id,
                owner_type: s.owner_type,
                owner_id: s.owner_id,
                created: s.created_at,
                updated: s.updated_at,
                compatibility: s.compatibility,
                dependencies: s.dependencies,
                content: s.content,
                install_count: s.install_count as u32,
                git_url: s.git_url,
                visibility: crate::models::skill_policy::Visibility::from(s.visibility.as_str()),
                tools: s.tools,
                review_status: s.review_status,
                reviewed_by: s.reviewed_by,
                reviewed_at: s.reviewed_at,
                review_comment: s.review_comment,
            });

            resolved.push(ResolvedSkill {
                skill_id: skill_id_str,
                skill: model_skill,
                state: serde_json::json!({}),
            });
        }

        Ok(resolved)
    }

    pub async fn get_skill_dependency_tree(&self, skill_id: &str) -> Result<DependencyTree, AppError> {
        let mut visited = HashMap::new();
        self.build_dependency_tree_impl(skill_id, &mut visited).await
    }

    async fn build_dependency_tree_impl(&self, skill_id: &str, visited: &mut HashMap<String, bool>) -> Result<DependencyTree, AppError> {
        let deps = self.context_repo.get_skill_dependencies(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut children = Vec::new();
        for dep in deps {
            if !visited.contains_key(&dep.dependency_skill_id) {
                visited.insert(dep.dependency_skill_id.clone(), true);
                let child_tree = Box::pin(self.build_dependency_tree_impl(&dep.dependency_skill_id, visited)).await?;
                children.push(child_tree);
            }
        }

        let skill = self.skill_repo.find_by_id(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(DependencyTree {
            skill_id: skill_id.to_string(),
            skill_name: skill.as_ref().map(|s| s.name.clone()).unwrap_or_default(),
            version: skill.as_ref().map(|s| s.version.clone()).unwrap_or_default(),
            children,
        })
    }

    pub async fn check_circular_dependency(&self, skill_id: &str) -> Result<bool, AppError> {
        let mut visited = HashMap::new();
        let mut stack = HashMap::new();

        self.find_cycle_impl(skill_id, &mut visited, &mut stack).await
    }

    async fn find_cycle_impl(&self, skill_id: &str, visited: &mut HashMap<String, bool>, stack: &mut HashMap<String, bool>) -> Result<bool, AppError> {
        visited.insert(skill_id.to_string(), true);
        stack.insert(skill_id.to_string(), true);

        let deps = self.context_repo.get_skill_dependencies(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        for dep in deps {
            if !visited.contains_key(&dep.dependency_skill_id) {
                if Box::pin(self.find_cycle_impl(&dep.dependency_skill_id, visited, stack)).await? {
                    return Ok(true);
                }
            } else if stack.contains_key(&dep.dependency_skill_id) {
                return Ok(true);
            }
        }

        stack.insert(skill_id.to_string(), false);
        Ok(false)
    }

    pub async fn get_missing_dependencies(&self, skill_id: &str) -> Result<Vec<String>, AppError> {
        let deps = self.context_repo.get_skill_dependencies(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let mut missing = Vec::new();
        for dep in deps {
            if !dep.is_optional {
                let skill = self.skill_repo.find_by_id(&dep.dependency_skill_id)
                    .await
                    .map_err(|e| AppError::InternalError(e.to_string()))?;

                if skill.is_none() {
                    missing.push(dep.dependency_skill_id);
                }
            }
        }

        Ok(missing)
    }

    pub async fn delete_skill_dependencies(&self, skill_id: &str) -> Result<(), AppError> {
        self.context_repo.delete_skill_dependencies(skill_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct DependencyTree {
    pub skill_id: String,
    pub skill_name: String,
    pub version: String,
    pub children: Vec<DependencyTree>,
}
