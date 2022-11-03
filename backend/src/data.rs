use futures_util::stream::FuturesOrdered;

use grafana_plugin_sdk::backend::{self, DataSourceInstanceSettings};

use crate::{
    path::{PathDisplay, QueryId},
    queries::{Query, TailTarget},
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
impl MaterializePlugin {
    async fn query_data_single(
        &self,
        datasource_instance_settings: &DataSourceInstanceSettings,
        query: &backend::DataQuery<Query>,
    ) -> Result<backend::DataResponse, Error> {
        let q = &query.query;
        let client = self.get_client(datasource_instance_settings).await?;
        let target = q.as_tail()?;
        let rows = target.select_all(&client).await?;
        let mut frame = rows_to_frame(&rows);

        if let TailTarget::Select { statement } = target {
            let query_id = QueryId::from_statement(statement);
            self.sql_queries
                .write()
                .await
                .insert(query_id, statement.clone());
        }

        let path = q.to_path();
        // Set the channel of the frame, indicating to Grafana that it should switch to
        // streaming.
        let channel = format!("ds/{}/{}", datasource_instance_settings.uid, path)
            .parse()
            .map_err(Error::CreatingChannel)?;
        frame.set_channel(channel);
        let frame = frame.check()?;

        Ok(backend::DataResponse::new(
            query.ref_id.clone(),
            vec![frame],
        ))
    }
}

#[backend::async_trait]
impl backend::DataService for MaterializePlugin {
    type Query = Query;
    type QueryError = QueryError;
    type Stream<'a> = backend::BoxDataResponseStream<'a, Self::QueryError>;

    async fn query_data<'stream, 'req: 'stream, 'slf: 'req>(
        &'slf self,
        request: &'req backend::QueryDataRequest<Self::Query>,
    ) -> Self::Stream<'stream> {
        let datasource_settings = request
            .plugin_context
            .datasource_instance_settings
            .as_ref()
            .ok_or(Error::MissingDatasource)
            .unwrap();
        Box::pin(
            request
                .queries
                .iter()
                .map(|x| async move {
                    self.query_data_single(datasource_settings, x)
                        .await
                        .map_err(|source| QueryError {
                            ref_id: x.ref_id.clone(),
                            source,
                        })
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }
}
