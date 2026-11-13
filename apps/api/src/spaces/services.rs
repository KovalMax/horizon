use sqlx::PgPool;
use uuid::Uuid;

use super::{repositories::PostgresSpacesRepository, responses::SpaceResponse};

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
}

pub enum CreateSpaceError {
    InvalidName,
    Unavailable(sqlx::Error),
}
