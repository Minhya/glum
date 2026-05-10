use crate::auth;
use std::sync::Arc;
use tonic::{Request, Status};

pub fn auth_interceptor(
    jwt_secret: Arc<String>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    move |mut req: Request<()>| {
        let token = auth::extract_token(&req)?;
        let claims = auth::verify_token(token, &jwt_secret)?;
        req.extensions_mut().insert(claims);
        Ok(req)
    }
}
