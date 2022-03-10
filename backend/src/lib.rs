mod convert;
mod data;
mod diagnostics;
mod error;
mod path;
mod resource;
mod stream;

use grafana_plugin_sdk::backend;
use serde::Deserialize;
use tokio_postgres::{Client, Config, NoTls};

use convert::rows_to_frame;
use error::{Error, Result};
use path::{Path, TailTarget};

#[derive(Clone, Debug, Default)]
pub struct MaterializePlugin;

impl MaterializePlugin {
    async fn get_client(
        &self,
        datasource_settings: &backend::DataSourceInstanceSettings,
    ) -> std::result::Result<Client, Error> {
        let settings: MaterializeDatasourceSettings =
            serde_json::from_value(datasource_settings.json_data.clone())
                .map_err(Error::InvalidDatasourceSettings)?;
        let (client, connection) = Config::new()
            .user(&settings.username)
            .host(&settings.host)
            .port(settings.port)
            .connect(NoTls)
            .await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        Ok(client)
    }
}

#[derive(Debug, Deserialize)]
struct MaterializeDatasourceSettings {
    host: String,
    port: u16,
    username: String,
}
