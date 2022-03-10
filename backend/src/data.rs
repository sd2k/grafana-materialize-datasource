use futures_util::{
    stream::{FuturesOrdered, FuturesUnordered},
    StreamExt,
};
use serde::Deserialize;

use grafana_plugin_sdk::backend;
use tokio_postgres::Client;

use crate::{rows_to_frame, Error, MaterializePlugin, Path, TailTarget};

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
    target: TailTarget,
}

async fn query_data_single(
    client: Client,
    uid: String,
    query: backend::DataQuery,
) -> Result<backend::DataResponse, Error> {
    let target = serde_json::from_value(query.json)
        .map(|req: MaterializeQueryDataRequest| req.target)
        .map_err(|e| Error::InvalidTailTarget(e.to_string()))?;
    let rows = target.select_all(&client).await?;
    let mut frame = rows_to_frame(rows);

    frame.set_channel(
        format!("ds/{}/{}", uid, Path::Tail(target))
            .parse()
            .expect("constructing channel"),
    );
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
        Box::pin(
            request
                .queries
                .into_iter()
                .zip(clients)
                .map(move |(x, client)| {
                    let ref_id = x.ref_id.clone();
                    let uid = datasource_settings.uid.clone();
                    async {
                        query_data_single(client.unwrap(), uid, x)
                            .await
                            .map_err(|source| QueryError { ref_id, source })
                    }
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }
}

