use std::{collections::HashMap, sync::Arc};

use futures_util::{
    stream::{FuturesOrdered, FuturesUnordered},
    StreamExt,
};
use serde::Deserialize;

use grafana_plugin_sdk::backend;
use tokio::sync::RwLock;
use tokio_postgres::Client;

use crate::{
    path::{self, PathDisplay},
    queries, request, rows_to_frame, Error, MaterializePlugin,
};

#[derive(Debug, thiserror::Error)]
#[error("Error querying backend for {}: {}", .ref_id, .source)]
pub struct QueryError {
    ref_id: String,
    source: Error,
}

impl backend::DataQueryError for QueryError {
    fn ref_id(self) -> String {
        self.ref_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct MaterializeQueryDataRequest {
    #[serde(flatten)]
    target: request::TailTarget,
}

async fn query_data_single(
    client: Client,
    uid: String,
    query: backend::DataQuery,
    queries: Arc<RwLock<HashMap<path::QueryId, queries::SelectStatement>>>,
) -> Result<backend::DataResponse, Error> {
    let target: queries::TailTarget = serde_json::from_value(query.json)
        .map(|req: MaterializeQueryDataRequest| req.target)
        .map_err(|e| Error::InvalidTailTarget(e.to_string()))?
        .into();
    let rows = target.select_all(&client).await?;
    let mut frame = rows_to_frame(rows);

    let path = path::Path::Tail(target.clone().into());
    if let queries::TailTarget::Select { statement } = target {
        // Eww, this should definitely be cleaned up.
        if let path::Path::Tail(path::TailTarget::Select { query_id }) = &path {
            queries.write().await.insert(query_id.clone(), statement);
        }
    }

    // Set the channel of the frame, indicating to Grafana that it should switch to
    // streaming.
    let channel = format!("ds/{}/{}", uid, path.to_path())
        .parse()
        .map_err(Error::CreatingChannel)?;
    frame.set_channel(channel);
    let frame = frame.check()?;

    Ok(backend::DataResponse::new(query.ref_id, vec![frame]))
}

#[backend::async_trait]
impl backend::DataService for MaterializePlugin {
    type QueryError = QueryError;
    type Stream = backend::BoxDataResponseStream<Self::QueryError>;

    async fn query_data(&self, request: backend::QueryDataRequest) -> Self::Stream {
        let datasource_settings = request
            .plugin_context
            .datasource_instance_settings
            .clone()
            .ok_or(Error::MissingDatasource)
            .unwrap();
        let clients: Vec<_> = request
            .queries
            .iter()
            .map(|_| self.get_client(&datasource_settings))
            .collect::<FuturesUnordered<_>>()
            .collect()
            .await;
        let queries = self.sql_queries.clone();
        Box::pin(
            request
                .queries
                .into_iter()
                .zip(clients)
                .map(move |(x, client)| {
                    let queries = queries.clone();
                    let ref_id = x.ref_id.clone();
                    let uid = datasource_settings.uid.clone();
                    async {
                        let client = client.map_err(|source| QueryError {
                            ref_id: ref_id.clone(),
                            source,
                        })?;
                        query_data_single(client, uid, x, queries)
                            .await
                            .map_err(|source| QueryError { ref_id, source })
                    }
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }
}
