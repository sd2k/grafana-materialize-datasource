mod convert;
mod data;
mod diagnostics;
mod error;
mod path;
mod queries;
mod request;
mod resource;
mod stream;

use std::{collections::HashMap, sync::Arc};

use grafana_plugin_sdk::backend;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio_postgres::{Client, Config, NoTls};

use convert::rows_to_frame;
use error::{Error, Result};

#[derive(Clone, Debug, Default)]
pub struct MaterializePlugin {
    sql_queries: Arc<RwLock<HashMap<path::QueryId, queries::SelectStatement>>>,
}

impl MaterializePlugin {
    async fn get_client(
        &self,
        datasource_settings: &backend::DataSourceInstanceSettings,
    ) -> Result<Client> {
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

    async fn target(&self, path: path::Path) -> Result<queries::TailTarget> {
        match path {
            path::Path::Tail(path::TailTarget::Select { query_id }) => self
                .sql_queries
                .read()
                .await
                .get(&query_id)
                .cloned()
                .map(|statement| queries::TailTarget::Select { statement })
                .ok_or_else(|| Error::InvalidTailTarget(query_id.into_inner())),
            path::Path::Tail(path::TailTarget::Relation { name }) => {
                Ok(queries::TailTarget::Relation { name: name.into() })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct MaterializeDatasourceSettings {
    host: String,
    port: u16,
    username: String,
}
