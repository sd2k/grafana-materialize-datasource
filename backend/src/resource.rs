use bytes::Bytes;
use futures_util::stream;
use grafana_plugin_sdk::backend;
use http::{Response, StatusCode};
use serde::Serialize;

use crate::{Error, MaterializePlugin};

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Path not found")]
    NotFound,

    #[error("Plugin error: {0}")]
    Plugin(#[from] Error),

    #[error("Missing datasource settings")]
    MissingDatasourceSettings,
}

#[derive(Debug, Serialize)]
pub struct JsonError {
    error: String,
}

impl backend::ErrIntoHttpResponse for ResourceError {
    fn into_http_response(self) -> Result<Response<Bytes>, Box<dyn std::error::Error>> {
        let status = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Plugin(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::MissingDatasourceSettings => StatusCode::BAD_REQUEST,
        };
        Ok(Response::builder().status(status).body(Bytes::from(
            serde_json::to_vec(&JsonError {
                error: self.to_string(),
            })
            .expect("valid JSON"),
        ))?)
    }
}

#[backend::async_trait]
impl backend::ResourceService for MaterializePlugin {
    type Error = ResourceError;

    type InitialResponse = Response<Bytes>;

    type Stream = backend::BoxResourceStream<Self::Error>;

    async fn call_resource(
        &self,
        request: backend::CallResourceRequest,
    ) -> Result<(Self::InitialResponse, Self::Stream), Self::Error> {
        match request.request.uri().path() {
            "/relations" => {}
            _ => return Err(ResourceError::NotFound),
        };
        let datasource_settings = request
            .plugin_context
            .and_then(|pc| pc.datasource_instance_settings)
            .ok_or(ResourceError::MissingDatasourceSettings)?;
        let client = self.get_client(&datasource_settings).await?;

        let rows = client
            .query(
                r#"
            SELECT DISTINCT mzr.name AS name
            FROM mz_catalog.mz_relations mzr
            JOIN mz_catalog.mz_schemas mzs ON mzr.schema_id = mzs.id
            WHERE database_id IS NOT NULL
            ORDER BY mzr.name
        "#,
                &[],
            )
            .await
            .map_err(Error::Connection)?;
        let names: Vec<&str> = rows.iter().map(|row| row.get("name")).collect();

        let initial_response =
            Response::new(Bytes::from(serde_json::to_vec(&names).expect("valid JSON")));
        Ok((initial_response, Box::pin(stream::empty())))
    }
}
