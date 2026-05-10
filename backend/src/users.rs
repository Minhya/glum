use crate::auth;
use crate::glum::users_server::Users as UsersTrait;
use crate::glum::*;
use sqlx::PgPool;
use tonic::{Request, Response, Status};

const DEFAULT_USER_LIMIT: i64 = 100;
const MAX_USER_LIMIT: i64 = 500;

pub struct Users {
    pool: PgPool,
}

impl Users {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    display_name: String,
}

fn db_error(operation: &str, error: sqlx::Error) -> Status {
    eprintln!("{operation}: {error:?}");
    Status::internal("Database error")
}

fn user_limit(limit: i32) -> Result<i64, Status> {
    if limit < 0 {
        return Err(Status::invalid_argument("Limit cannot be negative"));
    }

    Ok(if limit == 0 {
        DEFAULT_USER_LIMIT
    } else {
        i64::from(limit).min(MAX_USER_LIMIT)
    })
}

fn to_proto_user(row: UserRow) -> User {
    User {
        id: row.id,
        username: row.email.clone(),
        email: row.email,
        display_name: row.display_name,
    }
}

#[tonic::async_trait]
impl UsersTrait for Users {
    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<ListUsersResponse>, Status> {
        let current_user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let query = req.query.trim();
        let limit = user_limit(req.limit)?;

        let rows = if query.is_empty() {
            sqlx::query_as::<_, UserRow>(
                r#"SELECT id, email, display_name
                   FROM users
                   WHERE id <> $1
                   ORDER BY display_name, email
                   LIMIT $2"#,
            )
            .bind(&current_user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else {
            let pattern = format!("%{}%", query.to_lowercase());
            sqlx::query_as::<_, UserRow>(
                r#"SELECT id, email, display_name
                   FROM users
                   WHERE id <> $1
                     AND (
                         lower(email) LIKE $2
                         OR lower(display_name) LIKE $2
                         OR lower(id) LIKE $2
                     )
                   ORDER BY display_name, email
                   LIMIT $3"#,
            )
            .bind(&current_user_id)
            .bind(pattern)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| db_error("list users failed", e))?;

        Ok(Response::new(ListUsersResponse {
            users: rows.into_iter().map(to_proto_user).collect(),
        }))
    }
}
