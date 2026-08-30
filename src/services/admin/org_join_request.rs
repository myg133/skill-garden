//! Organization Join Request Service

use uuid::Uuid;

use crate::db::repositories::org_join_request::{
    OrgJoinRequest, OrgJoinRequestRepository, OrgJoinRequestWithIdentity,
};
use crate::models::error::AppError;

#[derive(Clone)]
pub struct OrgJoinRequestService {
    repo: OrgJoinRequestRepository,
}

impl std::fmt::Debug for OrgJoinRequestService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrgJoinRequestService").finish()
    }
}

impl OrgJoinRequestService {
    pub fn new(repo: OrgJoinRequestRepository) -> Self {
        Self { repo }
    }

    /// Create a new join request
    pub async fn create(
        &self,
        organization_id: Uuid,
        identity_id: Uuid,
        message: Option<String>,
    ) -> Result<OrgJoinRequest, AppError> {
        // Check if there's already a pending request
        if let Some(existing) = self
            .repo
            .find_pending_by_org_and_identity(organization_id, identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
        {
            return Err(AppError::ValidationError(format!(
                "A pending request already exists: {}",
                existing.id
            )));
        }

        self.repo
            .create(organization_id, identity_id, message)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Get a request by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<OrgJoinRequest>, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// List requests for an organization with optional status filter
    pub async fn list_by_org(
        &self,
        organization_id: Uuid,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrgJoinRequestWithIdentity>, AppError> {
        self.repo
            .find_by_org(organization_id, status, limit, offset)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Check if there's a pending request
    pub async fn has_pending_request(
        &self,
        organization_id: Uuid,
        identity_id: Uuid,
    ) -> Result<bool, AppError> {
        let pending = self
            .repo
            .find_pending_by_org_and_identity(organization_id, identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(pending.is_some())
    }

    /// Approve a request
    pub async fn approve(
        &self,
        request_id: Uuid,
        reviewer_id: Uuid,
    ) -> Result<OrgJoinRequest, AppError> {
        self.repo
            .update_status(request_id, "approved", reviewer_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Reject a request
    pub async fn reject(
        &self,
        request_id: Uuid,
        reviewer_id: Uuid,
    ) -> Result<OrgJoinRequest, AppError> {
        self.repo
            .update_status(request_id, "rejected", reviewer_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Cancel a pending request (by the requester)
    pub async fn cancel(&self, organization_id: Uuid, identity_id: Uuid) -> Result<(), AppError> {
        self.repo
            .delete_pending_by_org_and_identity(organization_id, identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// Count requests by status
    pub async fn count_by_status(
        &self,
        organization_id: Uuid,
        status: Option<&str>,
    ) -> Result<i64, AppError> {
        self.repo
            .count_by_org_and_status(organization_id, status)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }
}
