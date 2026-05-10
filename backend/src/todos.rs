use crate::auth;
use crate::calendar::{CalendarClient, CalendarTodo};
use crate::glum::todos_server::Todos as TodosTrait;
use crate::glum::*;
use prost_types::Timestamp;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use tonic::{Request, Response, Status};
use uuid::Uuid;

const MAX_TITLE_CHARS: usize = 100;
const MAX_USER_ID_CHARS: usize = 255;
const DEFAULT_PAGE_SIZE: i64 = 100;
const MAX_PAGE_SIZE: i64 = 500;

pub struct Todos {
    pool: PgPool,
    calendar: CalendarClient,
}

impl Todos {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            calendar: CalendarClient::new(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TodoRow {
    id: Uuid,
    title: String,
    completed: bool,
    priority: String,
    due_date: Option<OffsetDateTime>,
    due_has_time: bool,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    can_write: bool,
    owner: bool,
    calendar_sync_enabled: bool,
    calendar_sync_status: String,
    calendar_sync_error: Option<String>,
    google_calendar_event_id: Option<String>,
    google_calendar_event_link: Option<String>,
}

fn to_proto_todo(row: TodoRow) -> Todo {
    let priority = match row.priority.as_str() {
        "MEDIUM" => Priority::Medium,
        "HIGH" => Priority::High,
        _ => Priority::Low,
    };

    let to_timestamp = |dt: OffsetDateTime| Timestamp {
        seconds: dt.unix_timestamp(),
        nanos: dt.nanosecond() as i32,
    };

    Todo {
        id: row.id.to_string(),
        title: row.title,
        completed: row.completed,
        priority: priority as i32,
        due_date: row.due_date.map(to_timestamp),
        due_has_time: row.due_has_time,
        created_at: Some(to_timestamp(row.created_at)),
        updated_at: Some(to_timestamp(row.updated_at)),
        can_write: row.can_write,
        owner: row.owner,
        calendar_sync_enabled: row.calendar_sync_enabled,
        calendar_sync_status: row.calendar_sync_status,
        calendar_sync_error: row.calendar_sync_error.unwrap_or_default(),
        google_calendar_event_id: row.google_calendar_event_id.unwrap_or_default(),
        google_calendar_event_link: row.google_calendar_event_link.unwrap_or_default(),
    }
}

fn db_error(operation: &str, error: sqlx::Error) -> Status {
    eprintln!("{operation}: {error:?}");
    Status::internal("Database error")
}

fn validate_title(title: &str) -> Result<(), Status> {
    if title.trim().is_empty() {
        return Err(Status::invalid_argument("Title cannot be empty"));
    }

    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(Status::invalid_argument(
            "Title cannot exceed 100 characters",
        ));
    }

    Ok(())
}

fn validate_share_user_id(user_id: &str) -> Result<(), Status> {
    if user_id.trim().is_empty() {
        return Err(Status::invalid_argument("User ID cannot be empty"));
    }

    if user_id.chars().count() > MAX_USER_ID_CHARS {
        return Err(Status::invalid_argument("User ID is too long"));
    }

    Ok(())
}

fn parse_uuid(id: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(id).map_err(|_| Status::invalid_argument("Invalid UUID"))
}

fn priority_to_db(priority: i32) -> &'static str {
    match Priority::try_from(priority) {
        Ok(Priority::Medium) => "MEDIUM",
        Ok(Priority::High) => "HIGH",
        _ => "LOW",
    }
}

fn timestamp_to_offset(timestamp: Option<Timestamp>) -> Result<Option<OffsetDateTime>, Status> {
    let Some(timestamp) = timestamp else {
        return Ok(None);
    };

    if !(0..1_000_000_000).contains(&timestamp.nanos) {
        return Err(Status::invalid_argument("Invalid due date"));
    }

    let seconds = OffsetDateTime::from_unix_timestamp(timestamp.seconds)
        .map_err(|_| Status::invalid_argument("Invalid due date"))?;

    seconds
        .checked_add(Duration::nanoseconds(timestamp.nanos as i64))
        .map(Some)
        .ok_or_else(|| Status::invalid_argument("Invalid due date"))
}

fn pagination(limit: i32, offset: i32) -> Result<(i64, i64), Status> {
    if limit < 0 {
        return Err(Status::invalid_argument("Limit cannot be negative"));
    }

    if offset < 0 {
        return Err(Status::invalid_argument("Offset cannot be negative"));
    }

    let limit = if limit == 0 {
        DEFAULT_PAGE_SIZE
    } else {
        i64::from(limit).min(MAX_PAGE_SIZE)
    };

    Ok((limit, i64::from(offset)))
}

#[derive(sqlx::FromRow)]
struct TodoCalendarSyncRow {
    google_event_id: Option<String>,
}

impl Todos {
    async fn todo_for_user(&self, id: Uuid, user_id: &str) -> Result<TodoRow, Status> {
        sqlx::query_as::<_, TodoRow>(
            r#"SELECT t.id,
                      t.title,
                      t.completed,
                      t.priority::text AS priority,
                      t.due_date,
                      t.due_has_time,
                      t.created_at,
                      t.updated_at,
                      (t.owner_user_id = $2 OR COALESCE(ts.can_write, FALSE)) AS can_write,
                      (t.owner_user_id = $2) AS owner,
                      COALESCE(tcs.sync_enabled, FALSE) AS calendar_sync_enabled,
                      CASE
                          WHEN tcs.todo_id IS NULL THEN 'not_synced'
                          WHEN NOT tcs.sync_enabled THEN 'disabled'
                          ELSE tcs.sync_status
                      END AS calendar_sync_status,
                      tcs.sync_error AS calendar_sync_error,
                      tcs.google_event_id AS google_calendar_event_id,
                      tcs.google_event_link AS google_calendar_event_link
               FROM todos t
               LEFT JOIN todo_shares ts
                 ON ts.todo_id = t.id
                AND ts.user_id = $2
               LEFT JOIN todo_calendar_syncs tcs
                 ON tcs.todo_id = t.id
                AND tcs.user_id = $2
               WHERE t.id = $1
                 AND (t.owner_user_id = $2 OR ts.user_id IS NOT NULL)"#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("get todo failed", e))?
        .ok_or_else(|| Status::not_found("Todo not found"))
    }

    async fn sync_row(
        &self,
        id: Uuid,
        user_id: &str,
    ) -> Result<Option<TodoCalendarSyncRow>, Status> {
        sqlx::query_as::<_, TodoCalendarSyncRow>(
            r#"SELECT google_event_id
               FROM todo_calendar_syncs
               WHERE todo_id = $1 AND user_id = $2"#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("get todo calendar sync failed", e))
    }

    async fn store_calendar_sync_success(
        &self,
        todo_id: Uuid,
        user_id: &str,
        calendar_id: &str,
        google_event_id: &str,
        google_event_link: Option<&str>,
    ) -> Result<(), Status> {
        sqlx::query(
            r#"INSERT INTO todo_calendar_syncs (
                   todo_id,
                   user_id,
                   calendar_id,
                   google_event_id,
                   google_event_link,
                   sync_enabled,
                   sync_status,
                   sync_error,
                   synced_at,
                   updated_at
               )
               VALUES ($1, $2, $3, $4, $5, TRUE, 'synced', NULL, NOW(), NOW())
               ON CONFLICT (todo_id, user_id)
               DO UPDATE SET calendar_id = EXCLUDED.calendar_id,
                             google_event_id = EXCLUDED.google_event_id,
                             google_event_link = EXCLUDED.google_event_link,
                             sync_enabled = TRUE,
                             sync_status = 'synced',
                             sync_error = NULL,
                             synced_at = NOW(),
                             updated_at = NOW()"#,
        )
        .bind(todo_id)
        .bind(user_id)
        .bind(calendar_id)
        .bind(google_event_id)
        .bind(google_event_link)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| db_error("store todo calendar sync failed", e))
    }

    async fn store_calendar_sync_error(
        &self,
        todo_id: Uuid,
        user_id: &str,
        error: &str,
    ) -> Result<(), Status> {
        sqlx::query(
            r#"INSERT INTO todo_calendar_syncs (
                   todo_id,
                   user_id,
                   sync_enabled,
                   sync_status,
                   sync_error,
                   updated_at
               )
               VALUES ($1, $2, TRUE, 'error', $3, NOW())
               ON CONFLICT (todo_id, user_id)
               DO UPDATE SET sync_enabled = TRUE,
                             sync_status = 'error',
                             sync_error = EXCLUDED.sync_error,
                             updated_at = NOW()"#,
        )
        .bind(todo_id)
        .bind(user_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| db_error("store todo calendar sync error failed", e))
    }

    async fn disable_calendar_sync(
        &self,
        todo_id: Uuid,
        user_id: &str,
        error: Option<&str>,
    ) -> Result<(), Status> {
        sqlx::query(
            r#"INSERT INTO todo_calendar_syncs (
                   todo_id,
                   user_id,
                   sync_enabled,
                   sync_status,
                   sync_error,
                   updated_at
               )
               VALUES ($1, $2, FALSE, 'disabled', $3, NOW())
               ON CONFLICT (todo_id, user_id)
               DO UPDATE SET sync_enabled = FALSE,
                             sync_status = 'disabled',
                             sync_error = EXCLUDED.sync_error,
                             updated_at = NOW()"#,
        )
        .bind(todo_id)
        .bind(user_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| db_error("disable todo calendar sync failed", e))
    }

    async fn sync_calendar_for_todo(&self, user_id: &str, todo: &TodoRow) -> Result<(), Status> {
        let Some(due_date) = todo.due_date else {
            return Err(Status::invalid_argument(
                "Todo needs a due date before calendar sync",
            ));
        };

        let Some(google_refresh_token) = self.google_refresh_token(user_id).await? else {
            self.store_calendar_sync_error(
                todo.id,
                user_id,
                "Google Calendar is not connected. Log out and sign in with calendar permission.",
            )
            .await?;
            return Ok(());
        };

        let sync_result = self
            .calendar
            .upsert_event(
                &google_refresh_token,
                CalendarTodo {
                    id: todo.id,
                    title: &todo.title,
                    completed: todo.completed,
                    due_date,
                    due_has_time: todo.due_has_time,
                    google_event_id: todo.google_calendar_event_id.as_deref(),
                },
            )
            .await;

        match sync_result {
            Ok(event) => {
                self.store_calendar_sync_success(
                    todo.id,
                    user_id,
                    &event.calendar_id,
                    &event.google_event_id,
                    event.google_event_link.as_deref(),
                )
                .await
            }
            Err(error) => {
                self.store_calendar_sync_error(todo.id, user_id, error.user_message())
                    .await
            }
        }
    }

    async fn google_refresh_token(&self, user_id: &str) -> Result<Option<String>, Status> {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT google_refresh_token FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map(|token| token.flatten().filter(|token| !token.trim().is_empty()))
        .map_err(|e| db_error("get Google refresh token failed", e))
    }
}

#[tonic::async_trait]
impl TodosTrait for Todos {
    async fn create_todo(
        &self,
        request: Request<CreateTodoRequest>,
    ) -> Result<Response<Todo>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let CreateTodoRequest {
            title,
            priority,
            due_date,
            sync_to_calendar,
            due_has_time,
        } = request.into_inner();

        validate_title(&title)?;

        let priority = priority_to_db(priority);
        let due_date = timestamp_to_offset(due_date)?;

        let due_has_time = due_has_time && due_date.is_some();

        let row = sqlx::query_as::<_, TodoRow>(
            r#"INSERT INTO todos (owner_user_id, title, priority, due_date, due_has_time)
               VALUES ($1, $2, $3::text::priority, $4, $5)
               RETURNING id,
                         title,
                         completed,
                         priority::text AS priority,
                         due_date,
                         due_has_time,
                         created_at,
                         updated_at,
                         TRUE AS can_write,
                         TRUE AS owner,
                         FALSE AS calendar_sync_enabled,
                         'not_synced'::text AS calendar_sync_status,
                         NULL::text AS calendar_sync_error,
                         NULL::text AS google_calendar_event_id,
                         NULL::text AS google_calendar_event_link"#,
        )
        .bind(&user_id)
        .bind(&title)
        .bind(priority)
        .bind(due_date)
        .bind(due_has_time)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| db_error("create todo failed", e))?;

        if sync_to_calendar {
            self.sync_calendar_for_todo(&user_id, &row).await?;
            let row = self.todo_for_user(row.id, &user_id).await?;
            return Ok(Response::new(to_proto_todo(row)));
        }

        Ok(Response::new(to_proto_todo(row)))
    }

    async fn get_todo(&self, request: Request<GetTodoRequest>) -> Result<Response<Todo>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        let row = self.todo_for_user(id, &user_id).await?;

        Ok(Response::new(to_proto_todo(row)))
    }

    async fn list_todos(
        &self,
        request: Request<ListTodosRequest>,
    ) -> Result<Response<ListTodosResponse>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let (limit, offset) = pagination(req.limit, req.offset)?;

        let rows = sqlx::query_as::<_, TodoRow>(
            r#"SELECT t.id,
                      t.title,
                      t.completed,
                      t.priority::text AS priority,
                      t.due_date,
                      t.due_has_time,
                      t.created_at,
                      t.updated_at,
                      (t.owner_user_id = $1 OR COALESCE(ts.can_write, FALSE)) AS can_write,
                      (t.owner_user_id = $1) AS owner,
                      COALESCE(tcs.sync_enabled, FALSE) AS calendar_sync_enabled,
                      CASE
                          WHEN tcs.todo_id IS NULL THEN 'not_synced'
                          WHEN NOT tcs.sync_enabled THEN 'disabled'
                          ELSE tcs.sync_status
                      END AS calendar_sync_status,
                      tcs.sync_error AS calendar_sync_error,
                      tcs.google_event_id AS google_calendar_event_id,
                      tcs.google_event_link AS google_calendar_event_link
               FROM todos t
               LEFT JOIN todo_shares ts
                 ON ts.todo_id = t.id
                AND ts.user_id = $1
               LEFT JOIN todo_calendar_syncs tcs
                 ON tcs.todo_id = t.id
                AND tcs.user_id = $1
               WHERE t.owner_user_id = $1 OR ts.user_id IS NOT NULL
               ORDER BY t.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| db_error("list todos failed", e))?;

        Ok(Response::new(ListTodosResponse {
            todos: rows.into_iter().map(to_proto_todo).collect(),
        }))
    }

    async fn update_todo(
        &self,
        request: Request<UpdateTodoRequest>,
    ) -> Result<Response<Todo>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let UpdateTodoRequest {
            id,
            title,
            priority,
            due_date,
            due_has_time,
        } = request.into_inner();

        validate_title(&title)?;

        let id = parse_uuid(&id)?;
        let priority = priority_to_db(priority);
        let due_date = timestamp_to_offset(due_date)?;
        let due_has_time = due_has_time && due_date.is_some();

        let row = sqlx::query_as::<_, TodoRow>(
            r#"UPDATE todos
               SET title = $1,
                   priority = $2::text::priority,
                   due_date = $3,
                   due_has_time = $4,
                   updated_at = NOW()
               WHERE id = $5
                 AND (
                     owner_user_id = $6
                     OR EXISTS (
                         SELECT 1
                         FROM todo_shares
                         WHERE todo_shares.todo_id = todos.id
                           AND todo_shares.user_id = $6
                           AND todo_shares.can_write
                     )
                 )
               RETURNING id,
                         title,
                         completed,
                         priority::text AS priority,
                         due_date,
                         due_has_time,
                         created_at,
                         updated_at,
                         TRUE AS can_write,
                         (owner_user_id = $6) AS owner,
                         FALSE AS calendar_sync_enabled,
                         'not_synced'::text AS calendar_sync_status,
                         NULL::text AS calendar_sync_error,
                         NULL::text AS google_calendar_event_id,
                         NULL::text AS google_calendar_event_link"#,
        )
        .bind(&title)
        .bind(priority)
        .bind(due_date)
        .bind(due_has_time)
        .bind(id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("update todo failed", e))?
        .ok_or_else(|| Status::not_found("Todo not found"))?;

        let row = self.todo_for_user(row.id, &user_id).await?;
        if row.calendar_sync_enabled {
            self.sync_calendar_for_todo(&user_id, &row).await?;
            let row = self.todo_for_user(row.id, &user_id).await?;
            return Ok(Response::new(to_proto_todo(row)));
        }

        Ok(Response::new(to_proto_todo(row)))
    }

    async fn toggle_todo(
        &self,
        request: Request<ToggleTodoRequest>,
    ) -> Result<Response<Todo>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        let row = sqlx::query_as::<_, TodoRow>(
            r#"UPDATE todos
               SET completed = NOT completed, updated_at = NOW()
               WHERE id = $1
                 AND (
                     owner_user_id = $2
                     OR EXISTS (
                         SELECT 1
                         FROM todo_shares
                         WHERE todo_shares.todo_id = todos.id
                           AND todo_shares.user_id = $2
                           AND todo_shares.can_write
                     )
                 )
               RETURNING id,
                         title,
                         completed,
                         priority::text AS priority,
                         due_date,
                         due_has_time,
                         created_at,
                         updated_at,
                         TRUE AS can_write,
                         (owner_user_id = $2) AS owner,
                         FALSE AS calendar_sync_enabled,
                         'not_synced'::text AS calendar_sync_status,
                         NULL::text AS calendar_sync_error,
                         NULL::text AS google_calendar_event_id,
                         NULL::text AS google_calendar_event_link"#,
        )
        .bind(id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("toggle todo failed", e))?
        .ok_or_else(|| Status::not_found("Todo not found"))?;

        let row = self.todo_for_user(row.id, &user_id).await?;
        if row.calendar_sync_enabled {
            self.sync_calendar_for_todo(&user_id, &row).await?;
            let row = self.todo_for_user(row.id, &user_id).await?;
            return Ok(Response::new(to_proto_todo(row)));
        }

        Ok(Response::new(to_proto_todo(row)))
    }

    async fn delete_todo(
        &self,
        request: Request<DeleteTodoRequest>,
    ) -> Result<Response<DeleteTodoResponse>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        if let Some(sync) = self.sync_row(id, &user_id).await? {
            if let Some(event_id) = sync.google_event_id {
                if let Some(google_refresh_token) = self.google_refresh_token(&user_id).await? {
                    self.calendar
                        .delete_event(&google_refresh_token, &event_id)
                        .await
                        .map_err(|e| Status::failed_precondition(e.user_message().to_string()))?;
                }
            }
        }

        let result = sqlx::query("DELETE FROM todos WHERE id = $1 AND owner_user_id = $2")
            .bind(id)
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| db_error("delete todo failed", e))?;

        Ok(Response::new(DeleteTodoResponse {
            success: result.rows_affected() > 0,
        }))
    }

    async fn set_todo_calendar_sync(
        &self,
        request: Request<SetTodoCalendarSyncRequest>,
    ) -> Result<Response<Todo>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        let todo = self.todo_for_user(id, &user_id).await?;

        if req.sync_enabled {
            self.sync_calendar_for_todo(&user_id, &todo).await?;
        } else {
            let mut disable_error = None;
            if let Some(event_id) = todo.google_calendar_event_id.as_deref() {
                if let Some(google_refresh_token) = self.google_refresh_token(&user_id).await? {
                    if let Err(error) = self
                        .calendar
                        .delete_event(&google_refresh_token, event_id)
                        .await
                    {
                        disable_error = Some(format!(
                            "Disabled locally, but Google Calendar delete failed: {}",
                            error.user_message()
                        ));
                    }
                }
            }
            self.disable_calendar_sync(id, &user_id, disable_error.as_deref())
                .await?;
        }

        let row = self.todo_for_user(id, &user_id).await?;
        Ok(Response::new(to_proto_todo(row)))
    }

    async fn share_todo(
        &self,
        request: Request<ShareTodoRequest>,
    ) -> Result<Response<ShareTodoResponse>, Status> {
        let owner_user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_share_user_id(&req.user_id)?;

        if req.user_id == owner_user_id {
            return Err(Status::invalid_argument(
                "Cannot share a todo with its owner",
            ));
        }

        let id = parse_uuid(&req.id)?;

        let result = sqlx::query(
            r#"INSERT INTO todo_shares (todo_id, user_id, can_write)
               SELECT id, $2, $3
               FROM todos
               WHERE id = $1
                 AND owner_user_id = $4
               ON CONFLICT (todo_id, user_id)
               DO UPDATE SET can_write = EXCLUDED.can_write"#,
        )
        .bind(id)
        .bind(&req.user_id)
        .bind(req.can_write)
        .bind(&owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| db_error("share todo failed", e))?;

        if result.rows_affected() == 0 {
            return Err(Status::not_found("Todo not found"));
        }

        Ok(Response::new(ShareTodoResponse { success: true }))
    }

    async fn unshare_todo(
        &self,
        request: Request<UnshareTodoRequest>,
    ) -> Result<Response<ShareTodoResponse>, Status> {
        let owner_user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_share_user_id(&req.user_id)?;
        let id = parse_uuid(&req.id)?;

        let result = sqlx::query(
            r#"DELETE FROM todo_shares ts
               WHERE ts.todo_id = $1
                 AND ts.user_id = $2
                 AND EXISTS (
                     SELECT 1
                     FROM todos t
                     WHERE t.id = ts.todo_id
                       AND t.owner_user_id = $3
                 )"#,
        )
        .bind(id)
        .bind(&req.user_id)
        .bind(&owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| db_error("unshare todo failed", e))?;

        Ok(Response::new(ShareTodoResponse {
            success: result.rows_affected() > 0,
        }))
    }
}
