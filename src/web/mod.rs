use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router, Server,
};

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub enum Status {
    Good,
    Bad(crate::data::DataError),
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: Status,
    pub time: chrono::DateTime<Utc>,
    pub last_send: chrono::DateTime<Utc>,
}

async fn status() -> Json<StatusResponse> {
    let (last_error, last_send) = (crate::data::LAST_STATUS.lock().await).clone();
    let status = if let Some(x) = last_error {
        Status::Bad(x)
    } else {
        Status::Good
    };
    Json(StatusResponse {
        status,
        time: Utc::now(),
        last_send,
    })
}

#[derive(Deserialize)]
struct UpdateReportingIntervalQueryParam {
    interval: f64,
}

#[derive(Serialize)]
struct UpdateReportingIntervalResponse {
    error: Option<String>,
}

async fn update_reporting_interval(
    Query(i): Query<UpdateReportingIntervalQueryParam>,
) -> Json<UpdateReportingIntervalResponse> {
    tracing::warn!(i.interval, "Updating reporting interval!");
    match crate::CONFIG.update_reporting_interval(i.interval) {
        Ok(_) => {
            tracing::info!(i.interval, "Succesfully updated reporting interval!");
            return Json(UpdateReportingIntervalResponse { error: None });
        }
        Err(error) => {
            tracing::error!(%error, "Couldn't update reporting inteval!");
            return Json(UpdateReportingIntervalResponse {
                error: Some(error.to_string()),
            });
        }
    }
}

pub async fn web() {
    let app = Router::new()
        .route("/status", get(status))
        .route(
            "/update_reporting_interval",
            post(update_reporting_interval),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http());

    Server::bind(&crate::CONFIG.configuration().endpoint.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
