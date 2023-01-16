use std::borrow::Cow;

use axum::{
    extract::Query,
    routing::{get, post},
    Json, Router, Server,
    response::Html,
};

use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::data::Data;

async fn root() -> Html<&'static str> {
    Html(include_str!("../../webinterface.html"))
}

#[derive(Serialize)]
pub enum Status {
    Good,
    Bad(crate::data::DataError),
}

#[derive(Serialize)]
pub struct StatusResponse<'a> {
    pub status: Status,
    pub time: chrono::DateTime<Utc>,
    pub last_send: chrono::DateTime<Utc>,
    pub last_sent_data: Data,
    pub reporting_interval: f64,
    pub graphana_endpoint: Cow<'a, str>,
}

async fn status<'a>() -> Json<StatusResponse<'a>> {
    let (last_error, last_send, last_sent_data) = (crate::data::LAST_STATUS.lock().await).clone();
    let reporting_interval = *crate::CONFIG.configuration().reporting_interval.lock().unwrap();
    let graphana_endpoint = crate::CONFIG.configuration().graphana_endpoint.clone();
    let status = if let Some(x) = last_error {
        Status::Bad(x)
    } else {
        Status::Good
    };
    Json(StatusResponse {
        status,
        time: Utc::now().round_subsecs(3),
        last_send: last_send.round_subsecs(3),
        last_sent_data,
        reporting_interval,
        graphana_endpoint,
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
            return Json(UpdateReportingIntervalResponse { error: None })
        }
        Err(error) => {
            tracing::error!(%error, "Couldn't update reporting inteval!");
            return Json(UpdateReportingIntervalResponse {
                error: Some(error.to_string()),
            })
        }
    }
}

// Does not send back error, this is intended!
async fn force_send_data() {
    if crate::data::data_collection().await.is_err() {};
}

pub async fn web() {
    let app = Router::new()
        .route("/", get(root))
        .route("/status", get(status))
        .route(
            "/update_reporting_interval",
            post(update_reporting_interval),
        )
        .route("/force_send_data", post(force_send_data))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    Server::bind(&crate::CONFIG.configuration().endpoint.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
