use grafana_plugin_sdk::backend::{self, HealthStatus};
use serde_json::Value;

use crate::{Error, MaterializePlugin, Result};

impl MaterializePlugin {
    /// Connect to the database and run a `SELECT 1` query, returning `Ok(())` on
    /// success or `Err(Error)` if anything goes wrong.
    async fn check_health(&self, request: &backend::CheckHealthRequest) -> Result<()> {
        let datasource_settings = request
            .plugin_context
            .as_ref()
            .ok_or(Error::MissingDatasource)
            .and_then(|pc| {
                pc.datasource_instance_settings
                    .as_ref()
                    .ok_or(Error::MissingDatasource)
            })?;
        let client = self.get_client(datasource_settings).await?;
        Ok(client.query("SELECT 1", &[]).await.map(|_| ())?)
    }
}

#[backend::async_trait]
impl backend::DiagnosticsService for MaterializePlugin {
    type CheckHealthError = Error;

    async fn check_health(
        &self,
        request: backend::CheckHealthRequest,
    ) -> Result<backend::CheckHealthResponse> {
        match self.check_health(&request).await {
            Ok(_) => Ok(backend::CheckHealthResponse::new(
                HealthStatus::Ok,
                "Connection successful".to_string(),
                Value::Null,
            )),
            Err(e) => Ok(backend::CheckHealthResponse::new(
                HealthStatus::Error,
                e.to_string(),
                Value::Null,
            )),
        }
    }

    type CollectMetricsError = Error;

    async fn collect_metrics(
        &self,
        _request: backend::CollectMetricsRequest,
    ) -> Result<backend::CollectMetricsResponse> {
        todo!()
    }
}
