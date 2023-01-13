use axum::{
    Json,
    routing::get,
    Router,
    Server
};

use chrono::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub enum Status {
    Good,
    Bad,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: Status,
    pub time: chrono::DateTime<Utc>,
    pub last_send: chrono::DateTime<Utc>,
}

async fn status() -> Json<StatusResponse> {
    Json(StatusResponse { status: Status::Good, time: Utc::now(), last_send: DateTime::parse_from_rfc3339("2001-09-11T00:00:00Z").unwrap().with_timezone(&Utc) })
}

pub async fn web() {
    let app = Router::new()
    .route("/status", get(status))
    .layer(tower_http::trace::TraceLayer::new_for_http());

    Server::bind(&crate::CONFIG.configuration().endpoint.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
