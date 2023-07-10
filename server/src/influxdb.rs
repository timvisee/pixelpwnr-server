use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use futures::TryFutureExt;
use influxdb2::{
    models::{DataPoint, Query},
    FromDataPoint,
};
use serde::Deserialize;

use crate::stats::Stats;

#[derive(Clone, Debug, Deserialize)]
pub struct InfluxDBOptions {
    pub api_token: String,
    pub host: String,
    pub org: String,
    pub bucket: String,
    pub server_name: String,
}

pub struct InfluxDB {
    server_name: String,
    bucket: String,
    inner: influxdb2::Client,
}

impl From<InfluxDBOptions> for InfluxDB {
    fn from(value: InfluxDBOptions) -> Self {
        Self::new(value)
    }
}

impl InfluxDB {
    pub fn new(options: InfluxDBOptions) -> Self {
        let client = influxdb2::Client::new(options.host, options.org, options.api_token);

        Self {
            inner: client,
            server_name: options.server_name,
            bucket: options.bucket,
        }
    }

    pub async fn run(
        mut self,
        stats: Arc<Stats>,
        keep_running: Arc<AtomicBool>,
        interval: Duration,
    ) -> Result<(), String> {
        let mut reporting_interval = tokio::time::interval(interval);

        while keep_running.load(std::sync::atomic::Ordering::Relaxed) {
            reporting_interval.tick().await;
            self.write_stats(&stats).await?;
        }

        Ok(())
    }

    async fn load_stat(&mut self, stat_name: &str) -> Result<u64, String> {
        let Self {
            server_name,
            bucket,
            inner,
        } = self;

        let query = format!(
            r#"
            from(bucket: "{bucket}")
                |> range(start: -30d)
                |> filter(fn: (r) => r["_measurement"] == "{stat_name}")
                |> filter(fn: (r) => r["_field"] == "value")
                |> filter(fn: (r) => r["server_name"] == "{server_name}")
                |> max()
        "#
        );

        let query = Query::new(query);

        #[derive(FromDataPoint, Default)]
        struct Stat {
            value: f64,
        }

        let res: Vec<Stat> = inner.query::<Stat>(Some(query)).await.unwrap();

        Ok(res.first().map(|v| v.value).unwrap_or(0.0) as u64)
    }

    pub async fn load_stats(&mut self) -> Stats {
        macro_rules! try_load {
            ($name:literal) => {
                match self.load_stat($name).await {
                    Ok(loaded_value) => {
                        log::info!("Loaded stat {}. Value: {loaded_value}", $name);
                        loaded_value
                    }
                    Err(e) => {
                        log::warn!("Failed to load stat {}. {e}", $name);
                        0
                    }
                }
            };
        }

        let bandwidth = try_load!("bandwidth");
        let pixels_set = try_load!("pixels");

        Stats::new_with(pixels_set as usize, bandwidth as usize)
    }

    async fn write_stats(&mut self, stats: &Stats) -> Result<(), String> {
        let bandwidth_used = stats.bytes_read();
        let pixels_set = stats.pixels();
        let clients = stats.clients();

        log::debug!("Sending stats to influxdb:\n\t{bandwidth_used} bytes\n\t{pixels_set} pixels\n\t{clients} clients");

        let point = |name: &str, value: usize| {
            DataPoint::builder(name)
                .tag("server_name", &self.server_name)
                .field("value", value as f64)
                .build()
                .map_err(|e| format!("{e:?}"))
        };

        let points = vec![
            point("bandwidth", bandwidth_used)?,
            point("pixels", pixels_set)?,
            point("clients", clients)?,
        ];

        self.inner
            .write(&self.bucket, futures::stream::iter(points))
            .map_err(|e| format!("{e:?}"))
            .await?;

        Ok(())
    }
}
