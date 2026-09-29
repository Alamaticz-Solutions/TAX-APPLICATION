use appfw_saas_core::OffsetLimitContinuation;
use serde_json::Value;

use crate::AnaplanProviderError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanPagingSummary {
    pub current_page_size: u32,
    pub offset: u64,
    pub total_size: Option<u64>,
}

impl AnaplanPagingSummary {
    pub fn next_offset_continuation(&self, limit: u32) -> Option<OffsetLimitContinuation> {
        if limit == 0 || self.current_page_size == 0 {
            return None;
        }

        let next_offset = self.offset.saturating_add(self.current_page_size as u64);
        if self
            .total_size
            .map(|total_size| next_offset >= total_size)
            .unwrap_or(false)
        {
            return None;
        }

        let continuation = OffsetLimitContinuation::new(next_offset, limit);
        Some(match self.total_size {
            Some(total_size) => continuation.with_total_size(total_size),
            None => continuation,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnaplanCollectionSummary {
    pub item_count: u32,
    pub paging: Option<AnaplanPagingSummary>,
}

impl AnaplanCollectionSummary {
    pub fn from_json(value: &Value) -> Result<Self, AnaplanProviderError> {
        let item_count = value
            .get("items")
            .and_then(Value::as_array)
            .map(|items| items.len() as u32)
            .unwrap_or(0);

        let paging = value
            .pointer("/meta/paging")
            .map(parse_paging_summary)
            .transpose()?;

        Ok(Self { item_count, paging })
    }
}

fn parse_paging_summary(value: &Value) -> Result<AnaplanPagingSummary, AnaplanProviderError> {
    let object = value.as_object().ok_or_else(|| {
        AnaplanProviderError::InvalidResponse("meta.paging must be an object".to_string())
    })?;

    let current_page_size =
        required_u32(object.get("currentPageSize"), "meta.paging.currentPageSize")?;
    let offset = required_u64(object.get("offset"), "meta.paging.offset")?;
    let total_size = optional_u64(object.get("totalSize"), "meta.paging.totalSize")?;

    Ok(AnaplanPagingSummary {
        current_page_size,
        offset,
        total_size,
    })
}

fn required_u32(value: Option<&Value>, name: &'static str) -> Result<u32, AnaplanProviderError> {
    let value = required_u64(value, name)?;
    u32::try_from(value)
        .map_err(|_| AnaplanProviderError::InvalidResponse(format!("{name} must fit in u32")))
}

fn required_u64(value: Option<&Value>, name: &'static str) -> Result<u64, AnaplanProviderError> {
    value.and_then(Value::as_u64).ok_or_else(|| {
        AnaplanProviderError::InvalidResponse(format!("{name} must be an unsigned integer"))
    })
}

fn optional_u64(
    value: Option<&Value>,
    name: &'static str,
) -> Result<Option<u64>, AnaplanProviderError> {
    value
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                AnaplanProviderError::InvalidResponse(format!("{name} must be an unsigned integer"))
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collection_summary_parses_paging_without_raw_items() {
        let summary = AnaplanCollectionSummary::from_json(&json!({
            "meta": {
                "paging": {
                    "currentPageSize": 2,
                    "offset": 4,
                    "totalSize": 9
                }
            },
            "items": [
                { "id": "file1", "tokenValue": "secret" },
                { "id": "file2", "contents": "not retained" }
            ]
        }))
        .expect("summary");

        assert_eq!(summary.item_count, 2);
        assert_eq!(
            summary.paging,
            Some(AnaplanPagingSummary {
                current_page_size: 2,
                offset: 4,
                total_size: Some(9),
            })
        );
    }

    #[test]
    fn paging_summary_builds_offset_continuation() {
        let paging = AnaplanPagingSummary {
            current_page_size: 25,
            offset: 50,
            total_size: Some(100),
        };

        let continuation = paging
            .next_offset_continuation(25)
            .expect("next continuation");

        assert_eq!(continuation.offset, 75);
        assert_eq!(continuation.limit, 25);
        assert_eq!(continuation.total_size, Some(100));
        continuation.validate().expect("valid continuation");
    }

    #[test]
    fn paging_summary_omits_continuation_at_known_end() {
        let paging = AnaplanPagingSummary {
            current_page_size: 25,
            offset: 75,
            total_size: Some(100),
        };

        assert_eq!(paging.next_offset_continuation(25), None);
    }

    #[test]
    fn collection_summary_rejects_malformed_paging() {
        let error = AnaplanCollectionSummary::from_json(&json!({
            "meta": {
                "paging": {
                    "currentPageSize": "2",
                    "offset": 0
                }
            },
            "items": []
        }))
        .unwrap_err();

        assert!(error.to_string().contains("currentPageSize"));
    }
}
