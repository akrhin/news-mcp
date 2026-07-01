//! News poller implementation
//!
//! Background task that periodically fetches news from pluggable sources and updates cache.

use crate::cache::NewsCache;
use crate::config::PollerConfig;
use crate::service::NewsSource;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

/// News poller for background fetching
pub struct NewsPoller {
    /// Pluggable news sources
    sources: Vec<Arc<dyn NewsSource>>,
    /// Cache to store fetched articles
    cache: Arc<NewsCache>,
    /// Polling configuration
    config: PollerConfig,
    /// Running flag
    running: std::sync::atomic::AtomicBool,
    /// Initial poll completed flag
    initial_poll_completed: std::sync::atomic::AtomicBool,
}

impl NewsPoller {
    /// Create a new poller with the given sources
    pub fn new(
        sources: Vec<Arc<dyn NewsSource>>,
        cache: Arc<NewsCache>,
        config: PollerConfig,
    ) -> Self {
        Self {
            sources,
            cache,
            config,
            running: std::sync::atomic::AtomicBool::new(false),
            initial_poll_completed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Start background polling
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Poller is disabled by configuration");
            return;
        }

        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        info!(
            "Starting news poller with interval of {} seconds",
            self.config.interval_secs
        );

        // No initial poll on startup — server starts instantly.
        // First poll happens at the interval tick.
        // Set initial poll as "completed" immediately so startup is never blocked.
        self.initial_poll_completed
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Set up interval for subsequent polls
        let mut poll_interval = interval(Duration::from_secs(self.config.interval_secs));

        // First tick happens immediately (interval elapses right away)
        // but we skip it since we've already marked completed.
        // Actually: tokio::interval first tick IS immediate. We want first real poll at first tick.
        loop {
            poll_interval.tick().await;

            if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
                info!("Poller stopped");
                break;
            }

            if let Err(e) = self.poll_once().await {
                error!("Poll failed: {}", e);
            }
        }
    }

    /// Perform a single poll cycle across all registered sources
    pub async fn poll_once(&self) -> crate::error::Result<()> {
        info!("Starting poll cycle across {} sources", self.sources.len());
        let start_time = std::time::Instant::now();

        let mut total_articles = 0;
        let mut successful_categories = 0;

        for source in &self.sources {
            match source.fetch().await {
                Ok(results) => {
                    for (category, articles) in results {
                        let count = articles.len();
                        total_articles += count;

                        if count > 0 {
                            self.cache.set_category_news(category.clone(), articles)?;
                            successful_categories += 1;
                            info!(
                                "Updated {} articles for category {} from {}",
                                count,
                                category.clone(),
                                source.name()
                            );
                        } else {
                            warn!(
                                "No articles fetched for category {} from {}",
                                category.clone(),
                                source.name()
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Source '{}' fetch failed: {}", source.name(), e);
                }
            }
        }

        let elapsed = start_time.elapsed();
        info!(
            "Poll cycle completed: {} articles from {} categories in {}ms",
            total_articles,
            successful_categories,
            elapsed.as_millis()
        );

        Ok(())
    }

    /// Stop the poller
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        info!("Stopping news poller");
    }

    /// Check if poller is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Check if initial poll has completed
    pub fn is_initial_poll_completed(&self) -> bool {
        self.initial_poll_completed
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Wait for initial poll to complete (with timeout)
    pub async fn wait_for_initial_poll(&self, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        while !self
            .initial_poll_completed
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            if start.elapsed() > timeout {
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        true
    }
}
