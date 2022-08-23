use std::{collections::HashMap, sync::Arc};

use futures_util::{
    stream::{FuturesOrdered, FuturesUnordered},
    StreamExt,
};

use grafana_plugin_sdk::backend;
use tokio::sync::RwLock;
use tokio_postgres::Client;

use crate::{
    path::{self, PathDisplay, QueryId},
    queries::{Query, SelectStatement, TailTarget},
    rows_to_frame, Error, MaterializePlugin,
};

/// An error returned when querying for data.
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

/// Query for data for a single `DataQuery` in a request.
///
// Unfortunately this has to take all of its arguments by value until we have
// GATs, since the `DataService::Stream` associated type can't contain references.
// Ideally we'd just borrow the query/uid etc but it's really not a big deal.
async fn query_data_single(
    client: Client,
    uid: String,
    query: backend::DataQuery,
    queries: Arc<RwLock<HashMap<path::QueryId, SelectStatement>>>,
) -> Result<backend::DataResponse, Error> {
    let q: Query = serde_json::from_value(query.json).map_err(Error::InvalidQuery)?;
    let target = q.as_tail()?;
    let rows = target.select_all(&client).await?;
    let mut frame = rows_to_frame(&rows);

    if let TailTarget::Select { statement } = target {
        let query_id = QueryId::from_statement(statement);
        queries.write().await.insert(query_id, statement.clone());
    }

    let path = q.to_path();
    // Set the channel of the frame, indicating to Grafana that it should switch to
    // streaming.
    let channel = format!("ds/{}/{}", uid, path)
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
