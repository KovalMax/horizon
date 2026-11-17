use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::responses::{DeletedSpaceResponse, SpaceResponse, TransferOwnershipResponse};

async fn lock_owner_space_quota(
    transaction: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("active-owned-shared-spaces:{owner_id}"))
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn active_owned_shared_space_count(
    transaction: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT \
         FROM spaces \
         JOIN space_memberships ON space_memberships.space_id = spaces.id \
         WHERE spaces.kind = 'shared' AND spaces.deleted_at IS NULL \
           AND space_memberships.user_id = $1 AND space_memberships.role = 'owner'",
    )
    .bind(owner_id)
    .fetch_one(&mut **transaction)
    .await
}

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
             WHERE s.deleted_at IS NULL \
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
    ) -> Result<SpaceResponse, super::services::CreateSpaceError> {
        let mut transaction = self.pool.begin().await?;
        lock_owner_space_quota(&mut transaction, owner_id).await?;
        if active_owned_shared_space_count(&mut transaction, owner_id).await?
            >= super::services::MAX_ACTIVE_OWNED_SHARED_SPACES
        {
            return Err(super::services::CreateSpaceError::LimitReached);
        }
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

    pub(super) async fn transfer_ownership(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<TransferOwnershipResponse, super::services::SpaceLifecycleError> {
        use super::services::SpaceLifecycleError;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        let kind = sqlx::query_scalar::<_, String>(
            "SELECT kind FROM spaces WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(space_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?
        .ok_or(SpaceLifecycleError::NotFound)?;
        if kind != "shared" {
            return Err(SpaceLifecycleError::PersonalTransferUnsupported);
        }
        let owner_role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(space_id)
        .bind(owner_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        if owner_role.as_deref() != Some("owner") {
            return Err(if owner_role.is_some() {
                SpaceLifecycleError::OwnerRequired
            } else {
                SpaceLifecycleError::NotFound
            });
        }
        if owner_id == new_owner_id {
            return Err(SpaceLifecycleError::InvalidTransfer);
        }
        let target_role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(space_id)
        .bind(new_owner_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        if target_role.is_none() {
            return Err(SpaceLifecycleError::TargetNotMember);
        }
        let mut quota_owners = [owner_id, new_owner_id];
        quota_owners.sort_unstable();
        for quota_owner_id in quota_owners {
            lock_owner_space_quota(&mut transaction, quota_owner_id)
                .await
                .map_err(|_| SpaceLifecycleError::Unavailable)?;
        }
        if active_owned_shared_space_count(&mut transaction, new_owner_id)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?
            >= super::services::MAX_ACTIVE_OWNED_SHARED_SPACES
        {
            return Err(SpaceLifecycleError::LimitReached);
        }
        sqlx::query(
            "UPDATE space_memberships SET role = 'editor' WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(owner_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query(
            "UPDATE space_memberships SET role = 'owner' WHERE space_id = $1 AND user_id = $2",
        )
        .bind(space_id)
        .bind(new_owner_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        Ok(TransferOwnershipResponse {
            space_id,
            previous_owner_id: owner_id,
            new_owner_id,
        })
    }

    pub(super) async fn delete_space(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<DeletedSpaceResponse, super::services::SpaceLifecycleError> {
        use super::services::SpaceLifecycleError;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT name, kind FROM spaces WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(space_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?
        .ok_or(SpaceLifecycleError::NotFound)?;
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role FROM space_memberships WHERE space_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(space_id)
        .bind(owner_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        if role.as_deref() != Some("owner") {
            return Err(if role.is_some() {
                SpaceLifecycleError::OwnerRequired
            } else {
                SpaceLifecycleError::NotFound
            });
        }
        sqlx::query(
            "INSERT INTO deleted_space_memberships (space_id, user_id, role, membership_created_at) \
             SELECT space_id, user_id, role, created_at FROM space_memberships WHERE space_id = $1",
        )
        .bind(space_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query("DELETE FROM space_memberships WHERE space_id = $1")
            .bind(space_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query(
            "UPDATE space_invitations SET status = 'revoked' \
             WHERE space_id = $1 AND status = 'pending'",
        )
        .bind(space_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query(
            "UPDATE space_join_links SET revoked_at = $2 \
             WHERE space_id = $1 AND revoked_at IS NULL",
        )
        .bind(space_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query("UPDATE spaces SET deleted_at = $2, deleted_by = $3 WHERE id = $1")
            .bind(space_id)
            .bind(now)
            .bind(owner_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        Ok(DeletedSpaceResponse {
            id: space_id,
            name: row.0,
            kind: row.1,
            deleted_at: now.to_rfc3339(),
            restore_until: (now + Duration::days(7)).to_rfc3339(),
        })
    }

    pub(super) async fn list_deleted_for_owner(
        &self,
        owner_id: Uuid,
    ) -> Result<Vec<DeletedSpaceResponse>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>)>(
            "SELECT id, name, kind, deleted_at FROM spaces \
             WHERE deleted_by = $1 AND deleted_at > now() - interval '7 days' \
             ORDER BY deleted_at DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, name, kind, deleted_at)| DeletedSpaceResponse {
                id,
                name,
                kind,
                deleted_at: deleted_at.to_rfc3339(),
                restore_until: (deleted_at + Duration::days(7)).to_rfc3339(),
            })
            .collect())
    }

    pub(super) async fn restore_space(
        &self,
        space_id: Uuid,
        owner_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<SpaceResponse, super::services::SpaceLifecycleError> {
        use super::services::SpaceLifecycleError;

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        lock_owner_space_quota(&mut transaction, owner_id)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT name, kind FROM spaces \
             WHERE id = $1 AND deleted_by = $2 AND deleted_at > $3 - interval '7 days' \
             FOR UPDATE",
        )
        .bind(space_id)
        .bind(owner_id)
        .bind(now)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?
        .ok_or(SpaceLifecycleError::NotFound)?;
        if row.1 == "shared"
            && active_owned_shared_space_count(&mut transaction, owner_id)
                .await
                .map_err(|_| SpaceLifecycleError::Unavailable)?
                >= super::services::MAX_ACTIVE_OWNED_SHARED_SPACES
        {
            return Err(SpaceLifecycleError::LimitReached);
        }
        sqlx::query(
            "INSERT INTO space_memberships (space_id, user_id, role, created_at) \
             SELECT space_id, user_id, role, membership_created_at \
             FROM deleted_space_memberships WHERE space_id = $1",
        )
        .bind(space_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        let member_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM deleted_space_memberships WHERE space_id = $1",
        )
        .bind(space_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query("DELETE FROM deleted_space_memberships WHERE space_id = $1")
            .bind(space_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        sqlx::query("UPDATE spaces SET deleted_at = NULL, deleted_by = NULL WHERE id = $1")
            .bind(space_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        let owner_id_restored = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM space_memberships WHERE space_id = $1 AND role = 'owner'",
        )
        .bind(space_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| SpaceLifecycleError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SpaceLifecycleError::Unavailable)?;
        Ok(SpaceResponse {
            id: space_id,
            name: row.0,
            kind: row.1,
            role: if owner_id_restored == owner_id {
                "owner"
            } else {
                "viewer"
            }
            .to_owned(),
            member_count,
        })
    }

    pub(super) async fn purge_expired_deleted_spaces(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        Ok(
            sqlx::query("DELETE FROM spaces WHERE deleted_at <= $1 - interval '7 days'")
                .bind(now)
                .execute(&self.pool)
                .await?
                .rows_affected(),
        )
    }
}
