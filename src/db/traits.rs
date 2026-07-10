//! Repository traits for dependency injection

use crate::db::error::DbResult;
use crate::db::repositories::audit::{AuditLog, NewAuditLog};
use crate::db::repositories::evaluation::{Evaluation, NewEvaluation, SkillStats};
use crate::db::repositories::skill::{NewSkill, Skill, SkillMetadata};

#[allow(async_fn_in_trait)]
pub trait SkillRepositoryTrait: Send + Sync {
    async fn create(&self, new_skill: NewSkill) -> DbResult<Skill>;
    async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>>;
    async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>>;
    async fn count(&self) -> DbResult<i64>;
    async fn update(
        &self,
        skill_id: &str,
        description: Option<&str>,
        content: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> DbResult<()>;
    async fn delete(&self, skill_id: &str) -> DbResult<()>;
    async fn increment_install_count(&self, skill_id: &str) -> DbResult<()>;
}

#[allow(async_fn_in_trait)]
pub trait EvaluationRepositoryTrait: Send + Sync {
    async fn create(&self, new_eval: NewEvaluation) -> DbResult<Evaluation>;
    async fn get_stats(&self, skill_id: &str) -> DbResult<SkillStats>;
    async fn list_by_skill(&self, skill_id: &str, limit: i64) -> DbResult<Vec<Evaluation>>;
}

#[allow(async_fn_in_trait)]
pub trait AuditRepositoryTrait: Send + Sync {
    async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog>;
    async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>>;
}

impl<T: SkillRepositoryTrait + ?Sized> SkillRepositoryTrait for Box<T> {
    async fn create(&self, new_skill: NewSkill) -> DbResult<Skill> {
        (**self).create(new_skill).await
    }
    async fn find_by_id(&self, skill_id: &str) -> DbResult<Option<Skill>> {
        (**self).find_by_id(skill_id).await
    }
    async fn list(&self, limit: i64, offset: i64) -> DbResult<Vec<SkillMetadata>> {
        (**self).list(limit, offset).await
    }
    async fn count(&self) -> DbResult<i64> {
        (**self).count().await
    }
    async fn update(
        &self,
        skill_id: &str,
        description: Option<&str>,
        content: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> DbResult<()> {
        (**self).update(skill_id, description, content, tags).await
    }
    async fn delete(&self, skill_id: &str) -> DbResult<()> {
        (**self).delete(skill_id).await
    }
    async fn increment_install_count(&self, skill_id: &str) -> DbResult<()> {
        (**self).increment_install_count(skill_id).await
    }
}

impl<T: EvaluationRepositoryTrait + ?Sized> EvaluationRepositoryTrait for Box<T> {
    async fn create(&self, new_eval: NewEvaluation) -> DbResult<Evaluation> {
        (**self).create(new_eval).await
    }
    async fn get_stats(&self, skill_id: &str) -> DbResult<SkillStats> {
        (**self).get_stats(skill_id).await
    }
    async fn list_by_skill(&self, skill_id: &str, limit: i64) -> DbResult<Vec<Evaluation>> {
        (**self).list_by_skill(skill_id, limit).await
    }
}

impl<T: AuditRepositoryTrait + ?Sized> AuditRepositoryTrait for Box<T> {
    async fn create(&self, new_log: NewAuditLog) -> DbResult<AuditLog> {
        (**self).create(new_log).await
    }
    async fn list_by_agent(&self, agent_id: &str, limit: i64) -> DbResult<Vec<AuditLog>> {
        (**self).list_by_agent(agent_id, limit).await
    }
}
