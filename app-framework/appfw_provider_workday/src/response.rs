use crate::{xml, WorkdayProviderError};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkdayResponsePageSummary {
    pub total_results: Option<u32>,
    pub total_pages: Option<u32>,
    pub page_results: Option<u32>,
    pub page: Option<u32>,
}

impl WorkdayResponsePageSummary {
    pub fn from_response_xml(xml_body: &str) -> Result<Self, WorkdayProviderError> {
        let Some(response_results) = xml::extract_tag_text(xml_body, "Response_Results") else {
            return Ok(Self::default());
        };

        Ok(Self {
            total_results: parse_optional_u32(&response_results, "Total_Results")?,
            total_pages: parse_optional_u32(&response_results, "Total_Pages")?,
            page_results: parse_optional_u32(&response_results, "Page_Results")?,
            page: parse_optional_u32(&response_results, "Page")?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayServerTimestamp {
    pub value: String,
}

impl WorkdayServerTimestamp {
    pub fn from_response_xml(xml_body: &str) -> Result<Self, WorkdayProviderError> {
        let response = xml::extract_tag_text(xml_body, "Get_Server_Timestamp_Response")
            .ok_or_else(|| {
                WorkdayProviderError::InvalidResponse(
                    "Get_Server_Timestamp_Response is required".to_string(),
                )
            })?;
        let value = xml::extract_tag_text(&response, "Server_Timestamp").ok_or_else(|| {
            WorkdayProviderError::InvalidResponse("Server_Timestamp is required".to_string())
        })?;

        xml::validate_datetime_literal("Server_Timestamp", &value)?;

        Ok(Self { value })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayFreshnessSummary {
    pub page_summary: WorkdayResponsePageSummary,
    pub server_timestamp: Option<WorkdayServerTimestamp>,
}

impl WorkdayFreshnessSummary {
    pub fn from_response_xml(xml_body: &str) -> Result<Self, WorkdayProviderError> {
        let page_summary = WorkdayResponsePageSummary::from_response_xml(xml_body)?;
        let server_timestamp =
            if xml::extract_tag_text(xml_body, "Get_Server_Timestamp_Response").is_some() {
                Some(WorkdayServerTimestamp::from_response_xml(xml_body)?)
            } else {
                None
            };

        Ok(Self {
            page_summary,
            server_timestamp,
        })
    }
}

fn parse_optional_u32(
    xml_body: &str,
    tag_name: &'static str,
) -> Result<Option<u32>, WorkdayProviderError> {
    xml::extract_tag_text(xml_body, tag_name)
        .map(|value| {
            value.parse::<u32>().map_err(|_| {
                WorkdayProviderError::InvalidResponse(format!("{tag_name} must be a u32"))
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workday_response_page_summary() {
        let xml = r#"
            <bsvc:Response_Results>
                <bsvc:Total_Results>203</bsvc:Total_Results>
                <bsvc:Total_Pages>5</bsvc:Total_Pages>
                <bsvc:Page_Results>50</bsvc:Page_Results>
                <bsvc:Page>2</bsvc:Page>
            </bsvc:Response_Results>
        "#;

        let summary = WorkdayResponsePageSummary::from_response_xml(xml).expect("summary");

        assert_eq!(
            summary,
            WorkdayResponsePageSummary {
                total_results: Some(203),
                total_pages: Some(5),
                page_results: Some(50),
                page: Some(2),
            }
        );
    }

    #[test]
    fn rejects_non_numeric_response_page_summary() {
        let xml = "<bsvc:Response_Results><bsvc:Page>two</bsvc:Page></bsvc:Response_Results>";

        assert_eq!(
            WorkdayResponsePageSummary::from_response_xml(xml).unwrap_err(),
            WorkdayProviderError::InvalidResponse("Page must be a u32".to_string())
        );
    }

    #[test]
    fn scopes_page_summary_to_response_results() {
        let xml = r#"
            <bsvc:Worker>
                <bsvc:Page>999</bsvc:Page>
            </bsvc:Worker>
            <bsvc:Response_Results>
                <bsvc:Total_Results>2</bsvc:Total_Results>
                <bsvc:Total_Pages>1</bsvc:Total_Pages>
                <bsvc:Page_Results>2</bsvc:Page_Results>
                <bsvc:Page>1</bsvc:Page>
            </bsvc:Response_Results>
        "#;

        let summary = WorkdayResponsePageSummary::from_response_xml(xml).expect("summary");

        assert_eq!(summary.page, Some(1));
        assert_eq!(summary.total_results, Some(2));
    }

    #[test]
    fn parses_workday_server_timestamp() {
        let xml = r#"
            <bsvc:Get_Server_Timestamp_Response>
                <bsvc:Server_Timestamp>2026-06-26T04:00:00Z</bsvc:Server_Timestamp>
            </bsvc:Get_Server_Timestamp_Response>
        "#;

        let timestamp = WorkdayServerTimestamp::from_response_xml(xml).expect("timestamp");

        assert_eq!(timestamp.value, "2026-06-26T04:00:00Z");
    }

    #[test]
    fn rejects_missing_workday_server_timestamp() {
        let xml = "<bsvc:Get_Server_Timestamp_Response></bsvc:Get_Server_Timestamp_Response>";

        assert_eq!(
            WorkdayServerTimestamp::from_response_xml(xml).unwrap_err(),
            WorkdayProviderError::InvalidResponse("Server_Timestamp is required".to_string())
        );
    }

    #[test]
    fn server_timestamp_parser_scopes_to_timestamp_response() {
        let xml = r#"
            <bsvc:Get_Workers_Response>
                <bsvc:Server_Timestamp>2026-06-26T04:00:00Z</bsvc:Server_Timestamp>
            </bsvc:Get_Workers_Response>
        "#;

        assert_eq!(
            WorkdayServerTimestamp::from_response_xml(xml).unwrap_err(),
            WorkdayProviderError::InvalidResponse(
                "Get_Server_Timestamp_Response is required".to_string()
            )
        );
    }

    #[test]
    fn rejects_non_utc_workday_server_timestamp() {
        let xml = r#"
            <bsvc:Get_Server_Timestamp_Response>
                <bsvc:Server_Timestamp>2026-06-26T04:00:00-07:00</bsvc:Server_Timestamp>
            </bsvc:Get_Server_Timestamp_Response>
        "#;

        assert_eq!(
            WorkdayServerTimestamp::from_response_xml(xml).unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "Server_Timestamp",
                reason: "must be a UTC ISO-8601 datetime literal in YYYY-MM-DDTHH:MM:SSZ form"
            }
        );
    }

    #[test]
    fn freshness_summary_combines_page_and_server_timestamp_evidence() {
        let xml = r#"
            <bsvc:Get_Server_Timestamp_Response>
                <bsvc:Server_Timestamp>2026-06-26T04:00:00Z</bsvc:Server_Timestamp>
            </bsvc:Get_Server_Timestamp_Response>
            <bsvc:Response_Results>
                <bsvc:Total_Results>1</bsvc:Total_Results>
                <bsvc:Total_Pages>1</bsvc:Total_Pages>
                <bsvc:Page_Results>1</bsvc:Page_Results>
                <bsvc:Page>1</bsvc:Page>
            </bsvc:Response_Results>
        "#;

        let summary = WorkdayFreshnessSummary::from_response_xml(xml).expect("freshness");

        assert_eq!(summary.page_summary.total_results, Some(1));
        assert_eq!(
            summary
                .server_timestamp
                .as_ref()
                .map(|timestamp| timestamp.value.as_str()),
            Some("2026-06-26T04:00:00Z")
        );
    }
}
