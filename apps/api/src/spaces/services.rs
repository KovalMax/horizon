use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    repositories::PostgresSpacesRepository,
    responses::{DeletedSpaceResponse, SpaceResponse, TransferOwnershipResponse},
};

#[derive(Clone)]
pub struct SpacesApplication {
    repository: PostgresSpacesRepository,
}

impl SpacesApplication {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: PostgresSpacesRepository::new(pool),
        }
    }

    pub async fn list_for_member(&self, user_id: Uuid) -> Result<Vec<SpaceResponse>, sqlx::Error> {
        self.repository.list_for_member(user_id).await
    }

    pub async fn create_shared(
        &self,
        name: &str,
        owner_id: Uuid,
    ) -> Result<SpaceResponse, CreateSpaceError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err(CreateSpaceError::InvalidName);
        }
        self.repository
            .create_shared(name, owner_id)
            .await
            .map_err(CreateSpaceError::Unavailable)
    }

    pub async fn transfer_ownership(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<TransferOwnershipResponse, SpaceLifecycleError> {
        self.repository
            .transfer_ownership(space_id, owner_id, new_owner_id)
            .await
    }

    pub async fn delete_space(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
    ) -> Result<DeletedSpaceResponse, SpaceLifecycleError> {
        self.repository
            .delete_space(space_id, owner_id, Utc::now())
            .await
    }

    pub async fn list_deleted_for_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DeletedSpaceResponse>, sqlx::Error> {
        self.repository.list_deleted_for_owner(owner_id).await
    }

    pub async fn restore_space(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
    ) -> Result<SpaceResponse, SpaceLifecycleError> {
        self.repository
            .restore_space(space_id, owner_id, Utc::now())
            .await
    }

    pub async fn purge_expired_deleted_spaces(&self) -> Result<u64, sqlx::Error> {
        self.repository
            .purge_expired_deleted_spaces(Utc::now())
            .await
    }
}

pub enum CreateSpaceError {
    InvalidName,
    Unavailable(sqlx::Error),
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpaceLifecycleError {
    OwnerRequired,
    TargetNotMember,
    InvalidTransfer,
    PersonalTransferUnsupported,
    NotFound,
    Unavailable,
}
