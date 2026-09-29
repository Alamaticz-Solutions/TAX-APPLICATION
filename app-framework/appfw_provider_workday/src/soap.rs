use crate::{
    operation::push_optional_text, xml, WorkdayObjectReference, WorkdayProviderError,
    WorkdayResponseFilter, WorkdayWorkerTransactionLogCriteria,
};

pub const WORKDAY_WWS_VERSION: &str = "v46.1";
pub const WORKDAY_WWS_NAMESPACE: &str = "urn:com.workday/bsvc";
pub const SOAP_ENVELOPE_NAMESPACE: &str = "http://schemas.xmlsoap.org/soap/envelope/";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkdayWwsOperation {
    GetWorkers,
    GetFormerWorkers,
    GetWorkerEventHistory,
    GetOrganizations,
    GetLocations,
    GetJobProfiles,
    GetWorkdayAccount,
    GetServerTimestamp,
}

impl WorkdayWwsOperation {
    pub fn operation_name(self) -> &'static str {
        match self {
            Self::GetWorkers => "Get_Workers",
            Self::GetFormerWorkers => "Get_Former_Workers",
            Self::GetWorkerEventHistory => "Get_Worker_Event_History",
            Self::GetOrganizations => "Get_Organizations",
            Self::GetLocations => "Get_Locations",
            Self::GetJobProfiles => "Get_Job_Profiles",
            Self::GetWorkdayAccount => "Get_Workday_Account",
            Self::GetServerTimestamp => "Get_Server_Timestamp",
        }
    }

    pub fn request_element(self) -> &'static str {
        match self {
            Self::GetWorkers => "Get_Workers_Request",
            Self::GetFormerWorkers => "Get_Former_Workers_Request",
            Self::GetWorkerEventHistory => "Get_Worker_Event_History_Request",
            Self::GetOrganizations => "Get_Organizations_Request",
            Self::GetLocations => "Get_Locations_Request",
            Self::GetJobProfiles => "Get_Job_Profiles_Request",
            Self::GetWorkdayAccount => "Get_Workday_Account_Request",
            Self::GetServerTimestamp => "Get_Server_Timestamp_Request",
        }
    }
}

pub trait WorkdaySoapRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation;

    fn validate(&self) -> Result<(), WorkdayProviderError>;

    fn body_xml(&self) -> Result<String, WorkdayProviderError>;

    fn soap_envelope(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        Ok(format!(
            "<soapenv:Envelope xmlns:soapenv=\"{SOAP_ENVELOPE_NAMESPACE}\" xmlns:bsvc=\"{WORKDAY_WWS_NAMESPACE}\"><soapenv:Header/><soapenv:Body>{}</soapenv:Body></soapenv:Envelope>",
            self.body_xml()?
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkdayRequestTemplate {
    GetWorkers(GetWorkersRequestTemplate),
    GetWorkerEventHistory(GetWorkerEventHistoryRequestTemplate),
    GetOrganizations(GetOrganizationsRequestTemplate),
    GetLocations(GetLocationsRequestTemplate),
    GetJobProfiles(GetJobProfilesRequestTemplate),
    GetServerTimestamp(GetServerTimestampRequestTemplate),
}

impl WorkdaySoapRequestTemplate for WorkdayRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        match self {
            Self::GetWorkers(template) => template.wws_operation(),
            Self::GetWorkerEventHistory(template) => template.wws_operation(),
            Self::GetOrganizations(template) => template.wws_operation(),
            Self::GetLocations(template) => template.wws_operation(),
            Self::GetJobProfiles(template) => template.wws_operation(),
            Self::GetServerTimestamp(template) => template.wws_operation(),
        }
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        match self {
            Self::GetWorkers(template) => template.validate(),
            Self::GetWorkerEventHistory(template) => template.validate(),
            Self::GetOrganizations(template) => template.validate(),
            Self::GetLocations(template) => template.validate(),
            Self::GetJobProfiles(template) => template.validate(),
            Self::GetServerTimestamp(template) => template.validate(),
        }
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        match self {
            Self::GetWorkers(template) => template.body_xml(),
            Self::GetWorkerEventHistory(template) => template.body_xml(),
            Self::GetOrganizations(template) => template.body_xml(),
            Self::GetLocations(template) => template.body_xml(),
            Self::GetJobProfiles(template) => template.body_xml(),
            Self::GetServerTimestamp(template) => template.body_xml(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayWorkerResponseGroup {
    pub include_reference: bool,
    pub include_personal_information: bool,
    pub include_employment_information: bool,
    pub include_organizations: bool,
    pub include_roles: bool,
    pub include_compensation: bool,
    pub include_photos: bool,
    pub include_documents: bool,
}

impl WorkdayWorkerResponseGroup {
    pub fn least_data_worker_sync() -> Self {
        Self {
            include_reference: true,
            include_personal_information: false,
            include_employment_information: true,
            include_organizations: true,
            include_roles: false,
            include_compensation: false,
            include_photos: false,
            include_documents: false,
        }
    }

    fn to_xml(&self) -> String {
        let mut xml = String::from("<bsvc:Response_Group>");
        push_bool(&mut xml, "Include_Reference", self.include_reference);
        push_bool(
            &mut xml,
            "Include_Personal_Information",
            self.include_personal_information,
        );
        push_bool(
            &mut xml,
            "Include_Employment_Information",
            self.include_employment_information,
        );
        push_bool(
            &mut xml,
            "Include_Organizations",
            self.include_organizations,
        );
        push_bool(&mut xml, "Include_Roles", self.include_roles);
        push_bool(&mut xml, "Include_Compensation", self.include_compensation);
        push_bool(&mut xml, "Include_Photos", self.include_photos);
        push_bool(&mut xml, "Include_Documents", self.include_documents);
        xml.push_str("</bsvc:Response_Group>");
        xml
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetWorkersRequestTemplate {
    pub worker_references: Vec<WorkdayObjectReference>,
    pub transaction_log_criteria: WorkdayWorkerTransactionLogCriteria,
    pub response_filter: WorkdayResponseFilter,
    pub response_group: WorkdayWorkerResponseGroup,
}

impl GetWorkersRequestTemplate {
    pub fn list(
        transaction_log_criteria: WorkdayWorkerTransactionLogCriteria,
        response_filter: WorkdayResponseFilter,
    ) -> Self {
        Self {
            worker_references: Vec::new(),
            transaction_log_criteria,
            response_filter,
            response_group: WorkdayWorkerResponseGroup::least_data_worker_sync(),
        }
    }

    pub fn by_reference(
        worker_reference: WorkdayObjectReference,
        response_filter: WorkdayResponseFilter,
    ) -> Self {
        Self {
            worker_references: vec![worker_reference],
            transaction_log_criteria: WorkdayWorkerTransactionLogCriteria::default(),
            response_filter,
            response_group: WorkdayWorkerResponseGroup::least_data_worker_sync(),
        }
    }
}

impl WorkdaySoapRequestTemplate for GetWorkersRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetWorkers
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        self.response_filter.validate()?;
        self.transaction_log_criteria.validate()
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = request_open(self.wws_operation());

        if !self.worker_references.is_empty() {
            xml.push_str("<bsvc:Request_References>");
            for reference in &self.worker_references {
                xml.push_str(&reference.to_reference_xml("Worker_Reference")?);
            }
            xml.push_str("</bsvc:Request_References>");
        }

        if let Some(criteria_xml) = self.transaction_log_criteria.to_xml()? {
            xml.push_str("<bsvc:Request_Criteria>");
            xml.push_str(&criteria_xml);
            xml.push_str("</bsvc:Request_Criteria>");
        }

        xml.push_str(&self.response_filter.to_xml()?);
        xml.push_str(&self.response_group.to_xml());
        xml.push_str(&request_close(self.wws_operation()));

        Ok(xml)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkdayEventDateRange {
    pub from_event_date: String,
    pub through_event_date: String,
}

impl WorkdayEventDateRange {
    pub fn new(
        from_event_date: impl Into<String>,
        through_event_date: impl Into<String>,
    ) -> Result<Self, WorkdayProviderError> {
        let range = Self {
            from_event_date: from_event_date.into(),
            through_event_date: through_event_date.into(),
        };
        range.validate()?;
        Ok(range)
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        xml::validate_date_literal("from_event_date", &self.from_event_date)?;
        xml::validate_date_literal("through_event_date", &self.through_event_date)?;

        if self.from_event_date > self.through_event_date {
            return Err(WorkdayProviderError::InvalidParameter {
                name: "through_event_date",
                reason: "through date must not sort before from date",
            });
        }

        Ok(())
    }

    fn to_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = String::from("<bsvc:Event_Date_Range_Data>");
        push_optional_text(&mut xml, "From_Date", Some(&self.from_event_date));
        push_optional_text(&mut xml, "Through_Date", Some(&self.through_event_date));
        xml.push_str("</bsvc:Event_Date_Range_Data>");
        Ok(xml)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetWorkerEventHistoryRequestTemplate {
    pub worker_reference: WorkdayObjectReference,
    pub event_date_range: WorkdayEventDateRange,
    pub response_filter: WorkdayResponseFilter,
}

impl WorkdaySoapRequestTemplate for GetWorkerEventHistoryRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetWorkerEventHistory
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        self.response_filter.validate()?;
        self.event_date_range.validate()
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = request_open(self.wws_operation());
        xml.push_str("<bsvc:Request_Criteria>");
        xml.push_str(&self.worker_reference.to_reference_xml("Worker_Reference")?);
        xml.push_str(&self.event_date_range.to_xml()?);
        xml.push_str("</bsvc:Request_Criteria>");
        xml.push_str(&self.response_filter.to_xml()?);
        xml.push_str(&request_close(self.wws_operation()));

        Ok(xml)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetOrganizationsRequestTemplate {
    pub include_inactive: bool,
    pub response_filter: WorkdayResponseFilter,
}

impl WorkdaySoapRequestTemplate for GetOrganizationsRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetOrganizations
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        self.response_filter.validate()
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = request_open(self.wws_operation());
        xml.push_str("<bsvc:Request_Criteria>");
        push_bool(&mut xml, "Include_Inactive", self.include_inactive);
        xml.push_str("</bsvc:Request_Criteria>");
        xml.push_str(&self.response_filter.to_xml()?);
        xml.push_str(&request_close(self.wws_operation()));

        Ok(xml)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetLocationsRequestTemplate {
    pub include_inactive: bool,
    pub response_filter: WorkdayResponseFilter,
}

impl WorkdaySoapRequestTemplate for GetLocationsRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetLocations
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        self.response_filter.validate()
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = request_open(self.wws_operation());
        xml.push_str("<bsvc:Request_Criteria>");
        push_bool(&mut xml, "Include_Inactive", self.include_inactive);
        xml.push_str("</bsvc:Request_Criteria>");
        xml.push_str(&self.response_filter.to_xml()?);
        xml.push_str(&request_close(self.wws_operation()));

        Ok(xml)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetJobProfilesRequestTemplate {
    pub include_inactive: bool,
    pub response_filter: WorkdayResponseFilter,
}

impl WorkdaySoapRequestTemplate for GetJobProfilesRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetJobProfiles
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        self.response_filter.validate()
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        self.validate()?;

        let mut xml = request_open(self.wws_operation());
        xml.push_str("<bsvc:Request_Criteria>");
        push_bool(&mut xml, "Include_Inactive", self.include_inactive);
        xml.push_str("</bsvc:Request_Criteria>");
        xml.push_str(&self.response_filter.to_xml()?);
        xml.push_str(&request_close(self.wws_operation()));

        Ok(xml)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetServerTimestampRequestTemplate;

impl WorkdaySoapRequestTemplate for GetServerTimestampRequestTemplate {
    fn wws_operation(&self) -> WorkdayWwsOperation {
        WorkdayWwsOperation::GetServerTimestamp
    }

    fn validate(&self) -> Result<(), WorkdayProviderError> {
        Ok(())
    }

    fn body_xml(&self) -> Result<String, WorkdayProviderError> {
        Ok(format!(
            "{}{}",
            request_open(self.wws_operation()),
            request_close(self.wws_operation())
        ))
    }
}

fn request_open(operation: WorkdayWwsOperation) -> String {
    format!(
        "<bsvc:{} bsvc:version=\"{WORKDAY_WWS_VERSION}\">",
        operation.request_element()
    )
}

fn request_close(operation: WorkdayWwsOperation) -> String {
    format!("</bsvc:{}>", operation.request_element())
}

fn push_bool(xml: &mut String, element_name: &'static str, value: bool) {
    xml.push_str("<bsvc:");
    xml.push_str(element_name);
    xml.push('>');
    xml.push_str(if value { "true" } else { "false" });
    xml.push_str("</bsvc:");
    xml.push_str(element_name);
    xml.push('>');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WorkdayReferenceType, WorkdayResponseFilter};

    #[test]
    fn get_workers_template_builds_typed_soap_envelope() {
        let template = GetWorkersRequestTemplate::list(
            WorkdayWorkerTransactionLogCriteria {
                updated_from: Some("2026-06-25T00:00:00Z".to_string()),
                updated_through: Some("2026-06-26T00:00:00Z".to_string()),
                effective_from: None,
                effective_through: None,
            },
            WorkdayResponseFilter::new(1, 50)
                .with_as_of_effective_date("2026-06-26")
                .with_as_of_entry_datetime("2026-06-26T04:00:00Z"),
        );

        let envelope = template.soap_envelope().expect("envelope");

        assert!(envelope.contains("<soapenv:Envelope"));
        assert!(envelope.contains("<bsvc:Get_Workers_Request bsvc:version=\"v46.1\">"));
        assert!(envelope.contains("<bsvc:Updated_From>2026-06-25T00:00:00Z</bsvc:Updated_From>"));
        assert!(envelope.contains("<bsvc:Page>1</bsvc:Page><bsvc:Count>50</bsvc:Count>"));
        assert!(envelope.contains("<bsvc:Include_Photos>false</bsvc:Include_Photos>"));
        assert!(envelope.contains("<bsvc:Include_Documents>false</bsvc:Include_Documents>"));
    }

    #[test]
    fn get_worker_template_uses_request_references() {
        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let template = GetWorkersRequestTemplate::by_reference(
            reference,
            WorkdayResponseFilter::new(1, 1).with_as_of_entry_datetime("2026-06-26T04:00:00Z"),
        );

        let body = template.body_xml().expect("body");

        assert!(body.contains("<bsvc:Request_References>"));
        assert!(body.contains("<bsvc:ID bsvc:type=\"Employee_ID\">E123</bsvc:ID>"));
        assert!(body.contains("<bsvc:Count>1</bsvc:Count>"));
    }

    #[test]
    fn worker_event_history_template_requires_ordered_dates() {
        let reference =
            WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E123").unwrap();
        let event_date_range = WorkdayEventDateRange {
            from_event_date: "2026-06-27".to_string(),
            through_event_date: "2026-06-26".to_string(),
        };
        let template = GetWorkerEventHistoryRequestTemplate {
            worker_reference: reference,
            event_date_range,
            response_filter: WorkdayResponseFilter::default(),
        };

        assert_eq!(
            template.validate().unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "through_event_date",
                reason: "through date must not sort before from date"
            }
        );
    }

    #[test]
    fn get_server_timestamp_template_builds_empty_typed_request() {
        let template = GetServerTimestampRequestTemplate;

        let envelope = template.soap_envelope().expect("envelope");

        assert!(envelope.contains("<soapenv:Envelope"));
        assert!(envelope.contains(
            "<bsvc:Get_Server_Timestamp_Request bsvc:version=\"v46.1\"></bsvc:Get_Server_Timestamp_Request>"
        ));
    }
}
