use sqlx::PgPool;
use tonic::{Response,Request, Status};
use crate::glum::{CreateNoteRequest, notes_server::Notes as NotesTrait};
use crate::glum::*;

pub struct Notes {
    pool: PgPool,
}

impl Notes{
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
// service Notes {
//   rpc CreateNote(CreateNoteRequest) returns (Note);
//   rpc GetNote(GetNoteRequest) returns (Note);
//   rpc ListNotes(ListNotesRequest) returns (ListNotesResponse);
//   rpc UpdateNote(UpdateNoteRequest) returns (Note);
//   rpc DeleteNote(DeleteNoteRequest) returns (DeleteNoteResponse);
// }
#[tonic::async_trait]
impl NotesTrait for Notes {
    async fn create_note(&self, request: Request<CreateNoteRequest>) -> Result<Response<Note>, Status> {
        todo!()
    }
    async fn delete_note(&self, request: Request<DeleteNoteRequest>) -> Result<Response<DeleteNoteResponse>, Status> {
        todo!()
    }
    async fn get_note(&self, request: Request<GetNoteRequest>) -> Result<Response<Note>, Status> {
        todo!()
    }
    async fn list_notes(&self, request: Request<ListNotesRequest>) -> Result<Response<ListNotesResponse>, Status> {
        todo!()
    }
    async fn update_note(&self, request: Request<UpdateNoteRequest>) -> Result<Response<Note>, Status> {
        todo!()
    }

}


