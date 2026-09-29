use crate::{xml, WorkdayProviderError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkdayReferenceType {
    Wid,
    EmployeeId,
    ContingentWorkerId,
    OrganizationReferenceId,
    LocationId,
    JobProfileId,
}

impl WorkdayReferenceType {
    pub fn workday_id_type(self) -> &'static str {
        match self {
            Self::Wid => "WID",
            Self::EmployeeId => "Employee_ID",
            Self::ContingentWorkerId => "Contingent_Worker_ID",
            Self::OrganizationReferenceId => "Organization_Reference_ID",
            Self::LocationId => "Location_ID",
            Self::JobProfileId => "Job_Profile_ID",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkdayObjectReference {
    pub id_type: WorkdayReferenceType,
    pub value: String,
}

impl WorkdayObjectReference {
    pub fn new(
        id_type: WorkdayReferenceType,
        value: impl Into<String>,
    ) -> Result<Self, WorkdayProviderError> {
        let value = value.into();
        xml::validate_xml_value("reference.value", &value)?;

        Ok(Self { id_type, value })
    }

    pub(crate) fn to_reference_xml(
        &self,
        element_name: &'static str,
    ) -> Result<String, WorkdayProviderError> {
        xml::validate_xml_value("reference.value", &self.value)?;

        Ok(format!(
            "<bsvc:{element_name}><bsvc:ID bsvc:type=\"{}\">{}</bsvc:ID></bsvc:{element_name}>",
            self.id_type.workday_id_type(),
            xml::escape_xml_text(&self.value)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workday_reference_escapes_xml_text() {
        let reference = WorkdayObjectReference::new(WorkdayReferenceType::EmployeeId, "E<123>&A")
            .expect("reference");

        assert_eq!(
            reference
                .to_reference_xml("Worker_Reference")
                .expect("reference xml"),
            "<bsvc:Worker_Reference><bsvc:ID bsvc:type=\"Employee_ID\">E&lt;123&gt;&amp;A</bsvc:ID></bsvc:Worker_Reference>"
        );
    }

    #[test]
    fn workday_reference_rejects_empty_value() {
        assert_eq!(
            WorkdayObjectReference::new(WorkdayReferenceType::Wid, "").unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "reference.value",
                reason: "must not be empty"
            }
        );
    }
}
