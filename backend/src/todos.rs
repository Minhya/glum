use sqlx::PgPool;
use tonic::{Request, Response, Status};
use crate::glum::todos_server::Todos as TodosTrait;
use crate::glum::*;

pub struct Todos {
    pool: PgPool,
}

impl Todos {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[tonic::async_trait]
impl TodosTrait for Todos {
    async fn create_todo(&self, request: Request<CreateTodoRequest>) -> Result<Response<Todo>, Status> {
        todo!()
    }

    async fn get_todo(&self, request: Request<GetTodoRequest>) -> Result<Response<Todo>, Status> {
        todo!()
    }

    async fn list_todos(&self, request: Request<ListTodosRequest>) -> Result<Response<ListTodosResponse>, Status> {
        todo!()
    }

    async fn update_todo(&self, request: Request<UpdateTodoRequest>) -> Result<Response<Todo>, Status> {
        todo!()
    }

    async fn toggle_todo(&self, request: Request<ToggleTodoRequest>) -> Result<Response<Todo>, Status> {
        todo!()
    }

    async fn delete_todo(&self, request: Request<DeleteTodoRequest>) -> Result<Response<DeleteTodoResponse>, Status> {
        todo!()
    }
}
