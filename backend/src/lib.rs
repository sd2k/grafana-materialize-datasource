mod convert;
mod data;
mod diagnostics;
mod error;
mod path;
mod resource;
mod stream;

use grafana_plugin_sdk::backend;
use tokio_postgres::{Client, Config, NoTls};

use convert::rows_to_frame;
use error::{DatasourceSettingsError, Error, Result};
use path::{Path, TailTarget};

#[derive(Clone, Debug, Default)]
pub struct MaterializePlugin;

impl MaterializePlugin {
    async fn get_client(
        &self,
        datasource_settings: &backend::DataSourceInstanceSettings,
    ) -> std::result::Result<Client, Error> {
        let (client, connection) = Config::new()
            .user(&datasource_settings.user)
            .password(
                &datasource_settings
                    .decrypted_secure_json_data
                    .get("password")
                    .ok_or(DatasourceSettingsError::MissingPassword)?,
            )
            .host(
                datasource_settings
                    .json_data
                    .get("host")
                    .ok_or(DatasourceSettingsError::MissingHost)?
                    .as_str()
                    .ok_or(DatasourceSettingsError::InvalidHost)?,
            )
            .port(
                datasource_settings
                    .json_data
                    .get("port")
                    .ok_or(DatasourceSettingsError::MissingPort)?
                    .as_u64()
                    .and_then(|x| x.try_into().ok())
                    .ok_or(DatasourceSettingsError::InvalidPort)?,
            )
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
