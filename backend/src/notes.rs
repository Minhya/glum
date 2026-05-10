use crate::auth;
use crate::glum::notes_server::Notes as NotesTrait;
use crate::glum::*;
use prost_types::Timestamp;
use sqlx::PgPool;
use time::OffsetDateTime;
use tonic::{Request, Response, Status};
use uuid::Uuid;

const MAX_TITLE_CHARS: usize = 100;
const MAX_CONTENT_CHARS: usize = 10_000;
const MAX_USER_ID_CHARS: usize = 255;
const DEFAULT_PAGE_SIZE: i64 = 100;
const MAX_PAGE_SIZE: i64 = 500;

pub struct Notes {
    pool: PgPool,
}

impl Notes {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct NoteRow {
    id: Uuid,
    title: String,
    content: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    can_write: bool,
    owner: bool,
}

fn to_proto_note(row: NoteRow) -> Note {
    Note {
        id: row.id.to_string(),
        title: row.title,
        content: row.content,
        created_at: Some(Timestamp {
            seconds: row.created_at.unix_timestamp(),
            nanos: row.created_at.nanosecond() as i32,
        }),
        updated_at: Some(Timestamp {
            seconds: row.updated_at.unix_timestamp(),
            nanos: row.updated_at.nanosecond() as i32,
        }),
        can_write: row.can_write,
        owner: row.owner,
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

fn validate_content(content: &str) -> Result<(), Status> {
    if content.chars().count() > MAX_CONTENT_CHARS {
        return Err(Status::invalid_argument(
            "Content cannot exceed 10000 characters",
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

#[tonic::async_trait]
impl NotesTrait for Notes {
    async fn create_note(
        &self,
        request: Request<CreateNoteRequest>,
    ) -> Result<Response<Note>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_title(&req.title)?;
        validate_content(&req.content)?;

        let row = sqlx::query_as::<_, NoteRow>(
            r#"INSERT INTO notes (owner_user_id, title, content)
               VALUES ($1, $2, $3)
               RETURNING id, title, content, created_at, updated_at, TRUE AS can_write, TRUE AS owner"#,
        )
        .bind(&user_id)
        .bind(&req.title)
        .bind(&req.content)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| db_error("create note failed", e))?;

        Ok(Response::new(to_proto_note(row)))
    }

    async fn get_note(&self, request: Request<GetNoteRequest>) -> Result<Response<Note>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        let row = sqlx::query_as::<_, NoteRow>(
            r#"SELECT n.id,
                      n.title,
                      n.content,
                      n.created_at,
                      n.updated_at,
                      (n.owner_user_id = $2 OR COALESCE(ns.can_write, FALSE)) AS can_write,
                      (n.owner_user_id = $2) AS owner
               FROM notes n
               LEFT JOIN note_shares ns
                 ON ns.note_id = n.id
                AND ns.user_id = $2
               WHERE n.id = $1
                 AND (n.owner_user_id = $2 OR ns.user_id IS NOT NULL)"#,
        )
        .bind(id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("get note failed", e))?
        .ok_or_else(|| Status::not_found("Note not found"))?;

        Ok(Response::new(to_proto_note(row)))
    }

    async fn list_notes(
        &self,
        request: Request<ListNotesRequest>,
    ) -> Result<Response<ListNotesResponse>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let (limit, offset) = pagination(req.limit, req.offset)?;

        let rows = sqlx::query_as::<_, NoteRow>(
            r#"SELECT n.id,
                      n.title,
                      n.content,
                      n.created_at,
                      n.updated_at,
                      (n.owner_user_id = $1 OR COALESCE(ns.can_write, FALSE)) AS can_write,
                      (n.owner_user_id = $1) AS owner
               FROM notes n
               LEFT JOIN note_shares ns
                 ON ns.note_id = n.id
                AND ns.user_id = $1
               WHERE n.owner_user_id = $1 OR ns.user_id IS NOT NULL
               ORDER BY n.created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(&user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| db_error("list notes failed", e))?;

        Ok(Response::new(ListNotesResponse {
            notes: rows.into_iter().map(to_proto_note).collect(),
        }))
    }

    async fn update_note(
        &self,
        request: Request<UpdateNoteRequest>,
    ) -> Result<Response<Note>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_title(&req.title)?;
        validate_content(&req.content)?;

        let id = parse_uuid(&req.id)?;

        let row = sqlx::query_as::<_, NoteRow>(
            r#"UPDATE notes
               SET title = $1, content = $2, updated_at = NOW()
               WHERE id = $3
                 AND (
                     owner_user_id = $4
                     OR EXISTS (
                         SELECT 1
                         FROM note_shares
                         WHERE note_shares.note_id = notes.id
                           AND note_shares.user_id = $4
                           AND note_shares.can_write
                     )
                 )
               RETURNING id,
                         title,
                         content,
                         created_at,
                         updated_at,
                         TRUE AS can_write,
                         (owner_user_id = $4) AS owner"#,
        )
        .bind(&req.title)
        .bind(&req.content)
        .bind(id)
        .bind(&user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_error("update note failed", e))?
        .ok_or_else(|| Status::not_found("Note not found"))?;

        Ok(Response::new(to_proto_note(row)))
    }

    async fn delete_note(
        &self,
        request: Request<DeleteNoteRequest>,
    ) -> Result<Response<DeleteNoteResponse>, Status> {
        let user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;

        let result = sqlx::query("DELETE FROM notes WHERE id = $1 AND owner_user_id = $2")
            .bind(id)
            .bind(&user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| db_error("delete note failed", e))?;

        Ok(Response::new(DeleteNoteResponse {
            success: result.rows_affected() > 0,
        }))
    }

    async fn share_note(
        &self,
        request: Request<ShareNoteRequest>,
    ) -> Result<Response<ShareNoteResponse>, Status> {
        let owner_user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_share_user_id(&req.user_id)?;

        if req.user_id == owner_user_id {
            return Err(Status::invalid_argument(
                "Cannot share a note with its owner",
            ));
        }

        let id = parse_uuid(&req.id)?;

        let result = sqlx::query(
            r#"INSERT INTO note_shares (note_id, user_id, can_write)
               SELECT id, $2, $3
               FROM notes
               WHERE id = $1
                 AND owner_user_id = $4
               ON CONFLICT (note_id, user_id)
               DO UPDATE SET can_write = EXCLUDED.can_write"#,
        )
        .bind(id)
        .bind(&req.user_id)
        .bind(req.can_write)
        .bind(&owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| db_error("share note failed", e))?;

        if result.rows_affected() == 0 {
            return Err(Status::not_found("Note not found"));
        }

        Ok(Response::new(ShareNoteResponse { success: true }))
    }

    async fn unshare_note(
        &self,
        request: Request<UnshareNoteRequest>,
    ) -> Result<Response<ShareNoteResponse>, Status> {
        let owner_user_id = auth::current_user_id(&request)?.to_owned();
        let req = request.into_inner();

        validate_share_user_id(&req.user_id)?;
        let id = parse_uuid(&req.id)?;

        let result = sqlx::query(
            r#"DELETE FROM note_shares ns
               WHERE ns.note_id = $1
                 AND ns.user_id = $2
                 AND EXISTS (
                     SELECT 1
                     FROM notes n
                     WHERE n.id = ns.note_id
                       AND n.owner_user_id = $3
                 )"#,
        )
        .bind(id)
        .bind(&req.user_id)
        .bind(&owner_user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| db_error("unshare note failed", e))?;

        Ok(Response::new(ShareNoteResponse {
            success: result.rows_affected() > 0,
        }))
    }
}
