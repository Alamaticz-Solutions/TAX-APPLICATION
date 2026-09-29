use appfw_saas_core::OffsetLimitContinuation;
use serde_json::Value;

use crate::OracleFinancialsProviderError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleFinancialsCollectionSummary {
    pub count: u32,
    pub offset: u64,
    pub limit: u32,
    pub total_results: Option<u64>,
    pub has_more: Option<bool>,
}

impl OracleFinancialsCollectionSummary {
    pub fn from_json(value: &Value) -> Result<Self, OracleFinancialsProviderError> {
        let object = value
            .as_object()
            .ok_or(OracleFinancialsProviderError::InvalidResponse(
                "Oracle collection response must be a JSON object",
            ))?;

        let items = object.get("items").and_then(Value::as_array).ok_or(
            OracleFinancialsProviderError::InvalidResponse(
                "Oracle collection response must include items array",
            ),
        )?;

        let count = read_optional_u64(object.get("count"))
            .unwrap_or(items.len() as u64)
            .min(u32::MAX as u64) as u32;
        let offset = read_optional_u64(object.get("offset")).unwrap_or(0);
        let limit = read_optional_u64(object.get("limit"))
            .unwrap_or(count.max(1) as u64)
            .min(u32::MAX as u64) as u32;
        let total_results = read_optional_u64(object.get("totalResults"));
        let has_more = object.get("hasMore").and_then(Value::as_bool);

        if limit == 0 {
            return Err(OracleFinancialsProviderError::InvalidResponse(
                "Oracle collection limit must be greater than zero",
            ));
        }

        Ok(Self {
            count,
            offset,
            limit,
            total_results,
            has_more,
        })
    }

    pub fn next_offset_continuation(&self) -> Option<OffsetLimitContinuation> {
        let next_offset = self.offset.saturating_add(self.count as u64);

        let more_by_flag = self.has_more.unwrap_or(false);
        let more_by_total = self
            .total_results
            .is_some_and(|total_results| next_offset < total_results);

        if more_by_flag || more_by_total {
            let mut continuation = OffsetLimitContinuation::new(next_offset, self.limit);
            if let Some(total_results) = self.total_results {
                continuation = continuation.with_total_size(total_results);
            }
            return Some(continuation);
        }

        None
    }
}

fn read_optional_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collection_summary_parses_paging_without_retaining_items() {
        let value = json!({
            "items": [
                { "AccountingPeriodStatus": "OPEN", "sensitive": "payload" },
                { "AccountingPeriodStatus": "CLOSED", "sensitive": "payload" }
            ],
            "count": 2,
            "offset": 0,
            "limit": 2,
            "totalResults": 5,
            "hasMore": true
        });

        let summary = OracleFinancialsCollectionSummary::from_json(&value).expect("summary");

        assert_eq!(summary.count, 2);
        assert_eq!(summary.offset, 0);
        assert_eq!(summary.limit, 2);
        assert_eq!(summary.total_results, Some(5));
        assert_eq!(summary.has_more, Some(true));
    }

    #[test]
    fn collection_summary_builds_next_offset_continuation() {
        let summary = OracleFinancialsCollectionSummary {
            count: 25,
            offset: 50,
            limit: 25,
            total_results: Some(100),
            has_more: Some(true),
        };

        let continuation = summary
            .next_offset_continuation()
            .expect("next continuation");

        assert_eq!(continuation.offset, 75);
        assert_eq!(continuation.limit, 25);
        assert_eq!(continuation.total_size, Some(100));
    }

    #[test]
    fn collection_summary_omits_continuation_at_end() {
        let summary = OracleFinancialsCollectionSummary {
            count: 25,
            offset: 75,
            limit: 25,
            total_results: Some(100),
            has_more: Some(false),
        };

        assert_eq!(summary.next_offset_continuation(), None);
    }

    #[test]
    fn collection_summary_rejects_malformed_paging() {
        let error = OracleFinancialsCollectionSummary::from_json(&json!({
            "items": [],
            "limit": 0
        }))
        .unwrap_err();

        assert_eq!(
            error,
            OracleFinancialsProviderError::InvalidResponse(
                "Oracle collection limit must be greater than zero"
            )
        );
    }
}
