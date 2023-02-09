use std::fmt::Display;
use std::future::Future;
use tokio::time::{sleep, Duration};

pub struct ExponentialBackoff {
    sleep_time_secs: f64,
    factor: f64,
    retries: usize,
    max_retries: usize,
}

impl ExponentialBackoff {
    pub fn new(base_sleep_time_secs: f64, factor: f64, max_retries: usize) -> Self {
        Self {
            sleep_time_secs: base_sleep_time_secs,
            factor,
            retries: 0,
            max_retries,
        }
    }

    pub fn len(&self) -> usize {
        self.max_retries
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.retries > self.max_retries {
            return None;
        }
        let r = self.sleep_time_secs;
        self.sleep_time_secs *= self.factor;
        self.retries += 1;

        Some(Duration::from_secs_f64(r))
    }
}

pub async fn retry<Fut, R, E>(
    retry_name: &str,
    strategy: ExponentialBackoff,
    mut function: impl FnMut() -> Fut,
) -> Result<R, E>
where
    E: Display,
    Fut: Future<Output = Result<R, E>>,
{
    let retries = strategy.len();
    // Ok since we can only reach the bottom block, if this gets initialized.
    for (i, w) in strategy.enumerate() {
        let r = function().await;
        match r {
            Ok(x) => return Ok(x),
            Err(error) => {
                if i == retries {
                    return Err(error);
                }
                tracing::warn!(retry_name, %error, "Retrying!");
            }
        }
        sleep(w).await;
    }

    // Unrechable, since the for loop can only exit through a return.
    unreachable!();
}
