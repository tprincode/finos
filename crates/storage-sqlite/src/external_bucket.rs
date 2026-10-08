use application_core::contracts::{
    ExternalBudgetBucket, ExternalBudgetBucketListBody, ExternalBudgetBucketSave,
};
use application_core::ports::platform::PlatformError;
use sqlx::SqlitePool;
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage", err.to_string())
}

pub async fn list(pool: &SqlitePool) -> Result<ExternalBudgetBucketListBody, PlatformError> {
    let rows = sqlx::query_as::<_, BucketRow>(
        "SELECT bucket_id, name, bank, description, budget_category
         FROM external_budget_bucket
         ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    Ok(ExternalBudgetBucketListBody {
        buckets: rows.into_iter().map(|row| row.into_bucket()).collect(),
    })
}

pub async fn list_names(pool: &SqlitePool) -> Result<Vec<String>, PlatformError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM external_budget_bucket ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

pub async fn save(
    pool: &SqlitePool,
    incoming: ExternalBudgetBucketSave,
) -> Result<ExternalBudgetBucketListBody, PlatformError> {
    let name = incoming.name.trim().to_string();
    if name.is_empty() {
        return Err(PlatformError::new("bad_bucket", "Bucket name is required"));
    }
    let bucket_id = incoming.bucket_id.unwrap_or_else(Uuid::new_v4);
    let bank = incoming.bank.trim().to_string();
    let description = incoming.description.trim().to_string();
    let budget_category = if incoming.budget_category.trim().is_empty() {
        name.clone()
    } else {
        incoming.budget_category.trim().to_string()
    };

    let prior_name: Option<String> = if incoming.bucket_id.is_some() {
        sqlx::query_as::<_, (String,)>(
            "SELECT name FROM external_budget_bucket WHERE bucket_id = ?1",
        )
        .bind(bucket_id.to_string())
        .fetch_optional(pool)
        .await
        .map_err(map_sql)?
        .map(|(n,)| n)
    } else {
        None
    };

    let conflict: Option<(String,)> = sqlx::query_as(
        "SELECT bucket_id FROM external_budget_bucket
         WHERE name = ?1 COLLATE NOCASE AND bucket_id != ?2",
    )
    .bind(&name)
    .bind(bucket_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql)?;
    if conflict.is_some() {
        return Err(PlatformError::new(
            "duplicate_bucket",
            "A bucket with that name already exists",
        ));
    }

    sqlx::query(
        "INSERT INTO external_budget_bucket (bucket_id, name, bank, description, budget_category)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(bucket_id) DO UPDATE SET
            name = excluded.name,
            bank = excluded.bank,
            description = excluded.description,
            budget_category = excluded.budget_category",
    )
    .bind(bucket_id.to_string())
    .bind(&name)
    .bind(&bank)
    .bind(&description)
    .bind(&budget_category)
    .execute(pool)
    .await
    .map_err(map_sql)?;

    if let Some(old) = prior_name {
        let renamed = !old.is_empty() && !old.eq_ignore_ascii_case(&name);
        if renamed {
            sqlx::query(
                "UPDATE external_register_line
                 SET bucket = ?1
                 WHERE bucket = ?2 COLLATE NOCASE",
            )
            .bind(&name)
            .bind(&old)
            .execute(pool)
            .await
            .map_err(map_sql)?;
        }
    }

    list(pool).await
}

#[derive(sqlx::FromRow)]
struct BucketRow {
    bucket_id: String,
    name: String,
    bank: String,
    description: String,
    budget_category: String,
}

impl BucketRow {
    fn into_bucket(self) -> ExternalBudgetBucket {
        ExternalBudgetBucket {
            bucket_id: Uuid::parse_str(&self.bucket_id).unwrap_or_else(|_| Uuid::nil()),
            name: self.name,
            bank: self.bank,
            description: self.description,
            budget_category: self.budget_category,
        }
    }
}
