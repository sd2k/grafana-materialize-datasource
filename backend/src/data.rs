use futures_util::stream::FuturesOrdered;
use serde::Deserialize;

use grafana_plugin_sdk::{backend, data};

use crate::{Error, MaterializePlugin, Path, TailTarget};

// TODO(bsull) - make this error better and impl From so that query_data compiles.
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

impl MaterializePlugin {
    async fn query_data_single(
        &self,
        plugin_context: &backend::PluginContext,
        query: backend::DataQuery,
    ) -> Result<backend::DataResponse, Error> {
        let datasource_settings = plugin_context
            .datasource_instance_settings
            .as_ref()
            .ok_or(Error::MissingDatasource)?;
        let target = serde_json::from_value(query.json)
            .map(|req: MaterializeQueryDataRequest| req.target)
            .map_err(|e| Error::InvalidTailTarget(e.to_string()))?;
        let client = self.get_client(datasource_settings).await?;
        let rows = target.select_all(&client).await?;
        // TODO: use `rows` to create `frame`.
        let mut frame = data::Frame::new("");

        frame.set_channel(
            format!("ds/{}/{}", datasource_settings.uid, Path::Tail(target))
                .parse()
                .expect("constructing channel"),
        );
        let frame = frame.check()?;

        Ok(backend::DataResponse::new(query.ref_id, vec![frame]))
    }
}

#[backend::async_trait]
impl backend::DataService for MaterializePlugin {
    type QueryError = QueryError;
    type Stream<'a> = backend::BoxDataResponseStream<'a, Self::QueryError>;

    async fn query_data(&self, request: backend::QueryDataRequest) -> Self::Stream<'_> {
        let plugin_context = request.plugin_context;
        Box::pin(
            request
                .queries
                .into_iter()
                .map(|x| {
                    let ref_id = x.ref_id.clone();
                    let plugin_context = plugin_context.clone();
                    async move {
                        self.query_data_single(&plugin_context, x)
                            .await
                            .map_err(|source| QueryError { ref_id, source })
                    }
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }
}

// TODO(bsull): update these once we know what the request should look like.
// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn deserialize_request() {
//         assert_eq!(
//             serde_json::from_str::<MaterializeQueryDataRequest>(r#"{"path": "tasks"}"#).unwrap(),
//             MaterializeQueryDataRequest {
//                 target: "object",
//                 object: ""
//             }
//         );
//         assert_eq!(
//             serde_json::from_str::<MaterializeQueryDataRequest>(
//                 r#"{"path": "taskHistogram", "taskId": 1}"#
//             )
//             .unwrap(),
//             MaterializeQueryDataRequest {
//                 path: Path::TaskHistogram { task_id: TaskId(1) }
//             }
//         );
//         assert_eq!(
//             serde_json::from_str::<MaterializeQueryDataRequest>(r#"{"path": "resources"}"#)
//                 .unwrap(),
//             MaterializeQueryDataRequest {
//                 path: Path::Resources
//             }
//         );
//     }
// }
