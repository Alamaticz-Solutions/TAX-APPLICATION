use crate::xml;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayFaultKind {
    Authentication,
    Validation,
    Processing,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkdayFaultRetry {
    DoNotRetry,
    RetryReadWithBackoff,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayFaultClassification {
    pub kind: WorkdayFaultKind,
    pub retry: WorkdayFaultRetry,
    pub fault_code: Option<String>,
    pub message: Option<String>,
}

impl WorkdayFaultClassification {
    pub fn classify(fault_code: Option<&str>, message: Option<&str>) -> Self {
        let haystack = [fault_code.unwrap_or(""), message.unwrap_or("")]
            .join(" ")
            .to_ascii_lowercase();
        let kind = if haystack.contains("authentication_fault")
            || haystack.contains("authentication fault")
            || haystack.contains("auth")
        {
            WorkdayFaultKind::Authentication
        } else if haystack.contains("validation_fault") || haystack.contains("validation fault") {
            WorkdayFaultKind::Validation
        } else if haystack.contains("processing_fault") || haystack.contains("processing fault") {
            WorkdayFaultKind::Processing
        } else {
            WorkdayFaultKind::Unknown
        };

        let retry = match kind {
            WorkdayFaultKind::Processing => WorkdayFaultRetry::RetryReadWithBackoff,
            WorkdayFaultKind::Authentication
            | WorkdayFaultKind::Validation
            | WorkdayFaultKind::Unknown => WorkdayFaultRetry::DoNotRetry,
        };

        Self {
            kind,
            retry,
            fault_code: fault_code.map(str::to_string),
            message: message.map(str::to_string),
        }
    }

    pub fn from_soap_fault_xml(xml_body: &str) -> Self {
        let fault_code = xml::extract_tag_text(xml_body, "faultcode")
            .or_else(|| xml::extract_tag_text(xml_body, "Code"));
        let message = xml::extract_tag_text(xml_body, "faultstring")
            .or_else(|| xml::extract_tag_text(xml_body, "Reason"))
            .or_else(|| xml::extract_tag_text(xml_body, "message"));

        Self::classify(fault_code.as_deref(), message.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_authentication_faults_as_non_retryable() {
        let classification =
            WorkdayFaultClassification::classify(Some("bsvc:Authentication_Fault"), None);

        assert_eq!(classification.kind, WorkdayFaultKind::Authentication);
        assert_eq!(classification.retry, WorkdayFaultRetry::DoNotRetry);
    }

    #[test]
    fn classifies_processing_faults_as_read_retryable() {
        let classification =
            WorkdayFaultClassification::classify(None, Some("Processing_Fault timeout"));

        assert_eq!(classification.kind, WorkdayFaultKind::Processing);
        assert_eq!(
            classification.retry,
            WorkdayFaultRetry::RetryReadWithBackoff
        );
    }

    #[test]
    fn classifies_fault_from_soap_body() {
        let xml = r#"
            <soapenv:Fault>
                <faultcode>bsvc:Validation_Fault</faultcode>
                <faultstring>Invalid response filter</faultstring>
            </soapenv:Fault>
        "#;

        let classification = WorkdayFaultClassification::from_soap_fault_xml(xml);

        assert_eq!(classification.kind, WorkdayFaultKind::Validation);
        assert_eq!(classification.retry, WorkdayFaultRetry::DoNotRetry);
        assert_eq!(
            classification.fault_code.as_deref(),
            Some("bsvc:Validation_Fault")
        );
    }
}
