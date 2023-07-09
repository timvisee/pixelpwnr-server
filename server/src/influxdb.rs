use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use futures::TryFutureExt;
use influxdb2::models::DataPoint;

use crate::stats::Stats;

#[cfg(feature = "influxdb2")]
#[derive(Clone, Debug, clap::Args)]
pub struct InfluxDBOptions {
    #[clap(short = 'i', long)]
    pub run_influxdb: bool,
    #[clap(env = "INFLUXDB_TOKEN")]
    pub influxdb_api_token: String,
    #[clap(env = "INFLUXDB_HOST")]
    pub influxdb_host: String,
    #[clap(env = "INFLUXDB_ORG")]
    pub influxdb_org: String,
    #[clap(env = "INFLUXDB_BUCKET")]
    pub influxdb_bucket: String,
    #[clap(env = "INFLUXDB_SERVER_NAME", default_value = "pixelflut")]
    pub influxdb_server_name: String,
    #[clap(env = "INFLUXDB_REPORTING_INTERVAL_MS", default_value = "500")]
    pub influxdb_reporting_interval_ms: u64,
}

pub struct InfluxDb {
    server_name: String,
    bucket: String,
    stats: Arc<Stats>,
    reporting_interval: tokio::time::Interval,
    keep_running: Arc<AtomicBool>,
    inner: influxdb2::Client,
}

impl InfluxDb {
    pub async fn new(
        stats: Arc<Stats>,
        keep_running: Arc<AtomicBool>,
        options: InfluxDBOptions,
    ) -> Result<Self, String> {
        let client = influxdb2::Client::new(
            options.influxdb_host,
            options.influxdb_org,
            options.influxdb_api_token,
        );

        let mut me = Self {
            inner: client,
            server_name: options.influxdb_server_name,
            bucket: options.influxdb_bucket,
            reporting_interval: tokio::time::interval(Duration::from_millis(
                options.influxdb_reporting_interval_ms,
            )),
            stats,
            keep_running,
        };

        me.write_stats().await?;

        Ok(me)
    }

    pub async fn run(mut self) -> Result<(), String> {
        while self.keep_running.load(std::sync::atomic::Ordering::Relaxed) {
            self.reporting_interval.tick().await;
            self.write_stats().await?;
        }

        Ok(())
    }

    async fn write_stats(&mut self) -> Result<(), String> {
        let bandwidth_used = self.stats.bytes_read();
        let pixels_set = self.stats.pixels();
        let clients = self.stats.clients();

        println!("Writing stats... {bandwidth_used} bytes, {pixels_set} pixels, {clients} clients");

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
