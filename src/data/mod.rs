use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::Serialize;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Duration, Instant};

pub static LAST_STATUS: Lazy<Mutex<(Option<DataError>, DateTime<Utc>)>> =
    Lazy::new(|| Mutex::new((None, DateTime::default())));

#[derive(Error, Debug, Clone, Serialize)]
pub enum DataError {
    #[error("Command has Failed. {0}")]
    CommandFailed(String),
    #[error("Couldn't start the data gathering Command.")]
    CouldntStartCommand,
    #[error("Data gathering script provided invalid output!")]
    ScriptInvalidOutput,
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
                    },
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
        crate::retry::ExponentialBackoff::new(1.0 / 1000.0 * 100.0, 2.0, 3),
        collect,
    )
    .await;
    match data {
        Ok(x) => {
            *LAST_STATUS.lock().await = (None, Utc::now());
            tracing::info!(data = ?x, "Succesfully got data!");
            Ok(())
        }
        Err(x) => {
            tracing::error!("Couldn't get data: {x:#?}");
            *LAST_STATUS.lock().await = (Some(x.clone()), Utc::now());
            Err(x)
        }
    }
}

async fn collect() -> Result<(f64, f64), DataError> {
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

fn parse_script_output(mut output: String) -> Result<(f64, f64), DataError> {
    let len = output.trim_end_matches(&['\r', '\n'][..]).len();
    output.truncate(len);
    let segments = output.split(',').take(2).collect::<Vec<&str>>();
    let co2 = parse_as_f64(segments[0])?;
    let temperature = parse_as_f64(segments[1])?;

    Ok((co2, temperature))
}

fn parse_as_f64(s: &str) -> Result<f64, DataError> {
    match s.parse::<f64>() {
        Ok(x) => return Ok(x),
        Err(x) => {
            tracing::warn!("Couldn't parse as f64. {x}");
            match s.parse::<i32>() {
                Ok(x) => return Ok(x.into()),
                Err(x) => {
                    tracing::error!("Couldn't parse as f64 OR i32. {x} from {s}");
                    return Err(DataError::ScriptInvalidOutput);
                }
            }
        }
    }
}
