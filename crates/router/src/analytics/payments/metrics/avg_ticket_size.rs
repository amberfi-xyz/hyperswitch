use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::{PaymentMetric, PaymentMetricRow};
use crate::analytics::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct AvgTicketSize;

#[async_trait::async_trait]
impl<T> PaymentMetric<T> for AvgTicketSize
where
    T: AnalyticsDataSource + super::PaymentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentDimensions],
        merchant_id: &str,
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::Payment);

        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }

        query_builder
            .add_select_column(Aggregate::Sum {
                field: "amount",
                alias: Some("total"),
            })
            .switch()?;
        query_builder
            .add_select_column(Aggregate::Count {
                field: None,
                alias: Some("count"),
            })
            .switch()?;
        query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;
        query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;

        filters.set_filter_clause(&mut query_builder).switch()?;

        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        for dim in dimensions.iter() {
            query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        query_builder
            .execute_query::<PaymentMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        i.status.as_ref().map(|i| i.0),
                        i.connector.clone(),
                        i.authentication_type.as_ref().map(|i| i.0),
                        i.payment_method.clone(),
                        TimeRange {
                            start_time: match (granularity, i.start_bucket) {
                                (Some(g), Some(st)) => g.clip_to_start(st)?,
                                _ => time_range.start_time,
                            },
                            end_time: granularity.as_ref().map_or_else(
                                || Ok(time_range.end_time),
                                |g| i.end_bucket.map(|et| g.clip_to_end(et)).transpose(),
                            )?,
                        },
                    ),
                    i,
                ))
            })
            .collect::<error_stack::Result<
                Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>,
                crate::analytics::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
