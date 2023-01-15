use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::Serialize;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Duration, Instant};

use crate::retry;

#[derive(Default, Debug, Clone, Copy, Serialize)]
pub struct Data {
    co2: f64,
    temperature: f64,
}

#[allow(clippy::type_complexity)]
pub static LAST_STATUS: Lazy<Mutex<(Option<DataError>, DateTime<Utc>, Data)>> =
    Lazy::new(|| Mutex::new((None, DateTime::parse_from_rfc3339("1337-01-01T00:00:00Z").unwrap().with_timezone(&Utc), Data::default())));

#[derive(Error, Debug, Clone, Serialize)]
pub enum DataError {
    #[error("Command has Failed. {0}")]
    CommandFailed(String),
    #[error("Couldn't start the data gathering Command.")]
    CouldntStartCommand,
    #[error("Data gathering script provided invalid output!")]
    ScriptInvalidOutput,
    #[error("Sending data failed.")]
    SendDataFailed,
}

pub async fn data() {
    loop {
        let next_time = Instant::now()
            .checked_add(Duration::from_secs_f64(
                match crate::CONFIG.configuration().reporting_interval.lock() {
                    Ok(x) => {
                        let mut v = *x;
                        if v < 0.0 {
                            tracing::warn!(v, "Interval is negative using 0!");
                            v = 0.0;
                        }
                        v
                    }
                    Err(error) => {
                        tracing::error!(%error, "Couldn't lock interval.");
                        continue;
                    }
                },
            ))
            .unwrap();

        // Ignore errors since it's logged and we want to continue.
        if data_collection().await.is_err() {};

        sleep_until(next_time).await;
    }
}

pub async fn data_collection() -> Result<(), DataError> {
    let data = crate::retry::retry(
        "data",
        crate::retry::ExponentialBackoff::new((1.0 / 1000.0) * 100.0, 2.0, 3),
        collect,
    )
    .await;
    match data {
        Ok(x) => {
            *LAST_STATUS.lock().await = (None, Utc::now(), x);
            tracing::info!(data = ?x, "Successfully got data!");
            send_data(x).await?;
            Ok(())
        }
        Err(x) => {
            tracing::error!(error = %x, "Couldn't get data!");
            let mut last_status = LAST_STATUS.lock().await;
            *last_status = (Some(x.clone()), last_status.1, last_status.2);
            Err(x)
        }
    }
}

#[derive(Error, Debug)]
enum SendError {
    #[error("{0}")]
    Reqwest(reqwest::Error),
    #[error("{0}")]
    Other(String),
}

async fn send_data(data: Data) -> Result<(), DataError> {
    let client = reqwest::Client::new();
    match retry::retry("send_data", retry::ExponentialBackoff::new((1.0 / 1000.0) * 200.0, 2.0, 4), || async {
        let r = client.post(crate::CONFIG.configuration().node_endpoint.clone() + "/insert")
            .query(&[("kohlenstoff", data.co2.to_string()), ("temperatur", data.temperature.to_string()), ("raum_id", crate::CONFIG.configuration().raum_id.to_string())])
            .send()
            .await;
        match r {
            Ok(x) => {
                if x.status().is_success() {
                    return Ok(x)
                }
                return Err(SendError::Other(x.text().await.unwrap()));
            },
            Err(x) => return Err(SendError::Reqwest(x))
        }
    }).await {
        Ok(r) => {
            let resp = r.text().await.unwrap();
            tracing::info!(resp, "Succesfully sent data to server.")
        },
        Err(e) => {
            tracing::error!(%e, "Couldn't send data.");
            return Err(DataError::SendDataFailed)
        }
    }
    Ok(())
}

async fn collect() -> Result<Data, DataError> {
    let cmd = match Command::new("/usr/bin/python3")
        .arg(&crate::CONFIG.configuration().data_script)
        .output()
        .await
    {
        Ok(x) => x,
        Err(x) => {
            tracing::error!("Couldn't start data command: {x}");
            return Err(DataError::CouldntStartCommand);
        }
    };
    if !cmd.status.success() {
        return Err(DataError::CommandFailed(
            String::from_utf8(cmd.stderr).unwrap(),
        ));
    }

    let out = String::from_utf8(cmd.stdout).unwrap();

    parse_script_output(out)
}

fn parse_script_output(mut output: String) -> Result<Data, DataError> {
    let len = output.trim_end_matches(&['\r', '\n'][..]).len();
    output.truncate(len);
    let segments = output.split(',').take(2).collect::<Vec<&str>>();
    let co2 = parse_as_f64(segments[0])?;
    let temperature = parse_as_f64(segments[1])?;

    Ok(Data {co2, temperature})
}

fn parse_as_f64(s: &str) -> Result<f64, DataError> {
    match s.parse::<f64>() {
        Ok(x) => return Ok(x),
        Err(x) => {
            tracing::warn!("Couldn't parse as f64. {x}");
            match s.parse::<i32>() {
                Ok(x) => {
                    return Ok(x.into())
                },
                Err(x) => {
                    tracing::error!("Couldn't parse as f64 OR i32. {x} from {s}");
                    return Err(DataError::ScriptInvalidOutput);
                }
            }
        }
    }
}
