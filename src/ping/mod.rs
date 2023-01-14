use tokio::time::{sleep_until, Duration, Instant};

use reqwest::Client;

#[tracing::instrument]
pub async fn ping() {
    let c = match Client::builder().build() {
        Ok(x) => x,
        Err(x) => {
            tracing::error!("Failed to build client. {:?}", x);
            panic!("{}", x);
        }
    };
    loop {
        // Code to send and retry.
        let next_time = Instant::now().checked_add(Duration::from_secs(5)).unwrap();
        let ping_error = crate::retry::retry(
            "ping",
            crate::retry::ExponentialBackoff::new(1.0 / 1000.0 * 25.0, 2.0, 3),
            || async {
                return c
                    .post(crate::CONFIG.configuration().node_endpoint.clone() + "/ip_update")
                    .send()
                    .await;
            },
        )
        .await;
        if let Err(x) = ping_error {
            tracing::error!("Failed to ping. Last Error: {x}");
        }

        sleep_until(next_time).await;
    }
}