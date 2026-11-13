use sqlx::PgPool;
use uuid::Uuid;

use super::responses::SpaceResponse;

#[derive(Clone)]
pub(super) struct PostgresSpacesRepository {
    pool: PgPool,
}

impl PostgresSpacesRepository {
    pub(super) fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub(super) async fn list_for_member(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SpaceResponse>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String, i64)>(
            "SELECT s.id, s.name, s.kind, membership.role, COUNT(other_members.user_id)::BIGINT \
             FROM spaces s \
             JOIN space_memberships membership ON membership.space_id = s.id AND membership.user_id = $1 \
             LEFT JOIN space_memberships other_members ON other_members.space_id = s.id \
             GROUP BY s.id, membership.role \
             ORDER BY s.created_at, s.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, kind, role, member_count)| SpaceResponse {
                id,
                name,
                kind,
                role,
                member_count,
            })
            .collect())
    }

    pub(super) async fn create_shared(
        &self,
        name: &str,
        owner_id: Uuid,
    ) -> Result<SpaceResponse, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let space_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO spaces (id, name, kind, personal_owner_id) VALUES ($1, $2, 'shared', NULL)",
        )
        .bind(space_id)
        .bind(name)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "INSERT INTO space_memberships (space_id, user_id, role) VALUES ($1, $2, 'owner')",
        )
        .bind(space_id)
        .bind(owner_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        Ok(SpaceResponse {
            id: space_id,
            name: name.to_owned(),
            kind: "shared".to_owned(),
            role: "owner".to_owned(),
            member_count: 1,
        })
    }
}
