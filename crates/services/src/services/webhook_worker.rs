//! Background worker for processing pending webhook deliveries.
//!
//! This worker runs on a configurable interval and processes all pending
//! webhook deliveries, including retries for previously failed deliveries.

use std::time::Duration;

use db::DBService;
use tracing::{debug, error, info};

use crate::services::webhooks::WebhookService;

/// Default poll interval in seconds (30 seconds).
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;

/// Environment variable name for configuring the poll interval.
const POLL_INTERVAL_ENV_VAR: &str = "WEBHOOK_WORKER_POLL_INTERVAL_SECS";

/// Background worker service for processing webhook deliveries.
pub struct WebhookWorkerService {
    webhook_service: WebhookService,
    poll_interval: Duration,
}

impl WebhookWorkerService {
    /// Spawn the webhook worker as a background task.
    ///
    /// The poll interval can be configured via the `WEBHOOK_WORKER_POLL_INTERVAL_SECS`
    /// environment variable. Defaults to 30 seconds.
    ///
    /// Returns a JoinHandle for the spawned task.
    pub async fn spawn(db: DBService) -> tokio::task::JoinHandle<()> {
        let poll_interval = Self::get_poll_interval();
        let webhook_service = WebhookService::new(db.pool);

        let service = Self {
            webhook_service,
            poll_interval,
        };

        tokio::spawn(async move {
            service.start().await;
        })
    }

    /// Get the poll interval from environment variable or use default.
    fn get_poll_interval() -> Duration {
        std::env::var(POLL_INTERVAL_ENV_VAR)
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS))
    }

    /// Start the worker loop.
    async fn start(&self) {
        info!(
            poll_interval_secs = self.poll_interval.as_secs(),
            "Starting webhook worker service"
        );

        let mut interval = tokio::time::interval(self.poll_interval);

        loop {
            interval.tick().await;

            match self.process_deliveries().await {
                Ok((success_count, failure_count)) => {
                    if success_count > 0 || failure_count > 0 {
                        info!(
                            success_count,
                            failure_count,
                            "Processed webhook deliveries"
                        );
                    } else {
                        debug!("No pending webhook deliveries to process");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error processing webhook deliveries");
                }
            }
        }
    }

    /// Process all pending deliveries and return counts.
    ///
    /// Returns a tuple of (success_count, failure_count).
    async fn process_deliveries(&self) -> Result<(usize, usize), crate::services::webhooks::WebhookError> {
        let results = self.webhook_service.process_pending_deliveries().await?;

        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;

        Ok((success_count, failure_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_poll_interval() {
        // The default poll interval should be 30 seconds
        assert_eq!(DEFAULT_POLL_INTERVAL_SECS, 30);
    }

    #[test]
    fn test_poll_interval_parsing() {
        // Test that the parsing logic works correctly
        // (We can't safely set env vars in tests, so we test the parsing logic directly)
        let parse_interval = |s: &str| -> Option<Duration> {
            s.parse::<u64>().ok().map(Duration::from_secs)
        };

        assert_eq!(parse_interval("60"), Some(Duration::from_secs(60)));
        assert_eq!(parse_interval("1"), Some(Duration::from_secs(1)));
        assert_eq!(parse_interval("3600"), Some(Duration::from_secs(3600)));
        assert_eq!(parse_interval("not_a_number"), None);
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("-1"), None); // negative numbers fail u64 parse
    }
}
