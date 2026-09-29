use crate::SalesforceProviderError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SalesforceObject {
    Account,
    CareProgram,
    CareProgramEnrollee,
    ClinicalEncounter,
    CoverageBenefitItem,
}

impl SalesforceObject {
    pub fn api_name(self) -> &'static str {
        match self {
            Self::Account => "Account",
            Self::CareProgram => "CareProgram",
            Self::CareProgramEnrollee => "CareProgramEnrollee",
            Self::ClinicalEncounter => "ClinicalEncounter",
            Self::CoverageBenefitItem => "CoverageBenefitItem",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SalesforceField {
    AccountId,
    AccountName,
    AccountNumber,
    AccountSystemModstamp,
    AccountLastModifiedDate,
    AccountIsDeleted,
}

impl SalesforceField {
    pub fn object(self) -> SalesforceObject {
        match self {
            Self::AccountId
            | Self::AccountName
            | Self::AccountNumber
            | Self::AccountSystemModstamp
            | Self::AccountLastModifiedDate
            | Self::AccountIsDeleted => SalesforceObject::Account,
        }
    }

    pub fn api_name(self) -> &'static str {
        match self {
            Self::AccountId => "Id",
            Self::AccountName => "Name",
            Self::AccountNumber => "AccountNumber",
            Self::AccountSystemModstamp => "SystemModstamp",
            Self::AccountLastModifiedDate => "LastModifiedDate",
            Self::AccountIsDeleted => "IsDeleted",
        }
    }

    fn accepts_operator(self, operator: SalesforceOperator) -> bool {
        match self {
            Self::AccountId => matches!(operator, SalesforceOperator::Eq | SalesforceOperator::In),
            Self::AccountName | Self::AccountNumber | Self::AccountIsDeleted => {
                operator == SalesforceOperator::Eq
            }
            Self::AccountSystemModstamp | Self::AccountLastModifiedDate => matches!(
                operator,
                SalesforceOperator::Eq
                    | SalesforceOperator::Gt
                    | SalesforceOperator::Gte
                    | SalesforceOperator::Lt
                    | SalesforceOperator::Lte
            ),
        }
    }

    fn accepts_literal(self, literal: &SalesforceLiteral) -> bool {
        matches!(
            (self, literal),
            (Self::AccountId, SalesforceLiteral::Id(_))
                | (Self::AccountId, SalesforceLiteral::IdSet(_))
                | (
                    Self::AccountName | Self::AccountNumber,
                    SalesforceLiteral::Text(_)
                )
                | (
                    Self::AccountSystemModstamp | Self::AccountLastModifiedDate,
                    SalesforceLiteral::DateTime(_),
                )
                | (Self::AccountIsDeleted, SalesforceLiteral::Bool(_))
        )
    }

    fn accepts_filter_pair(
        self,
        operator: SalesforceOperator,
        literal: &SalesforceLiteral,
    ) -> bool {
        matches!(
            (self, operator, literal),
            (
                Self::AccountId,
                SalesforceOperator::Eq,
                SalesforceLiteral::Id(_)
            ) | (
                Self::AccountId,
                SalesforceOperator::In,
                SalesforceLiteral::IdSet(_)
            ) | (
                Self::AccountName | Self::AccountNumber,
                SalesforceOperator::Eq,
                SalesforceLiteral::Text(_),
            ) | (
                Self::AccountSystemModstamp | Self::AccountLastModifiedDate,
                SalesforceOperator::Eq
                    | SalesforceOperator::Gt
                    | SalesforceOperator::Gte
                    | SalesforceOperator::Lt
                    | SalesforceOperator::Lte,
                SalesforceLiteral::DateTime(_),
            ) | (
                Self::AccountIsDeleted,
                SalesforceOperator::Eq,
                SalesforceLiteral::Bool(_),
            )
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SalesforceOperator {
    Eq,
    Gt,
    Gte,
    In,
    Lt,
    Lte,
}

impl SalesforceOperator {
    pub fn soql(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::In => "IN",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SalesforceSortDirection {
    Asc,
    Desc,
}

impl SalesforceSortDirection {
    pub fn soql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SalesforceLiteral {
    Bool(bool),
    DateTime(String),
    Id(String),
    IdSet(Vec<String>),
    Text(String),
}

impl SalesforceLiteral {
    fn validate(&self) -> Result<(), SalesforceProviderError> {
        match self {
            Self::Bool(_) => Ok(()),
            Self::DateTime(value) => validate_datetime_literal(value),
            Self::Id(value) => validate_salesforce_id(value),
            Self::IdSet(values) => validate_salesforce_ids(values),
            Self::Text(_) => Ok(()),
        }
    }

    fn to_soql(&self) -> Result<String, SalesforceProviderError> {
        self.validate()?;

        Ok(match self {
            Self::Bool(value) => value.to_string(),
            Self::DateTime(value) => value.clone(),
            Self::Id(value) | Self::Text(value) => format!("'{}'", escape_soql_string(value)),
            Self::IdSet(values) => {
                let values = values
                    .iter()
                    .map(|value| format!("'{}'", escape_soql_string(value)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({values})")
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceFilter {
    pub field: SalesforceField,
    pub operator: SalesforceOperator,
    pub literal: SalesforceLiteral,
}

impl SalesforceFilter {
    pub fn new(
        field: SalesforceField,
        operator: SalesforceOperator,
        literal: SalesforceLiteral,
    ) -> Result<Self, SalesforceProviderError> {
        if !field.accepts_operator(operator) {
            return Err(SalesforceProviderError::InvalidSoql(format!(
                "operator {} is not registered for {}",
                operator.soql(),
                field.api_name()
            )));
        }

        if !field.accepts_literal(&literal) {
            return Err(SalesforceProviderError::InvalidSoql(format!(
                "literal type is not registered for {}",
                field.api_name()
            )));
        }

        if !field.accepts_filter_pair(operator, &literal) {
            return Err(SalesforceProviderError::InvalidSoql(format!(
                "operator {} is not registered with this literal type for {}",
                operator.soql(),
                field.api_name()
            )));
        }

        literal.validate()?;
        Ok(Self {
            field,
            operator,
            literal,
        })
    }

    fn to_soql(&self) -> Result<String, SalesforceProviderError> {
        Ok(format!(
            "{} {} {}",
            self.field.api_name(),
            self.operator.soql(),
            self.literal.to_soql()?
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SalesforceSort {
    pub field: SalesforceField,
    pub direction: SalesforceSortDirection,
}

impl SalesforceSort {
    pub fn ascending(field: SalesforceField) -> Self {
        Self {
            field,
            direction: SalesforceSortDirection::Asc,
        }
    }

    pub fn descending(field: SalesforceField) -> Self {
        Self {
            field,
            direction: SalesforceSortDirection::Desc,
        }
    }

    fn to_soql(self) -> String {
        format!("{} {}", self.field.api_name(), self.direction.soql())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SalesforceSoqlQuery {
    object: SalesforceObject,
    selected_fields: Vec<SalesforceField>,
    filters: Vec<SalesforceFilter>,
    order_by: Vec<SalesforceSort>,
    limit: Option<u32>,
}

impl SalesforceSoqlQuery {
    pub fn builder(object: SalesforceObject) -> Self {
        Self {
            object,
            selected_fields: Vec::new(),
            filters: Vec::new(),
            order_by: Vec::new(),
            limit: None,
        }
    }

    pub fn select(mut self, fields: impl IntoIterator<Item = SalesforceField>) -> Self {
        self.selected_fields.extend(fields);
        self
    }

    pub fn filter(mut self, filter: SalesforceFilter) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn order_by(mut self, sort: SalesforceSort) -> Self {
        self.order_by.push(sort);
        self
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn to_soql(&self) -> Result<String, SalesforceProviderError> {
        self.validate()?;

        let selected_fields = self
            .selected_fields
            .iter()
            .map(|field| field.api_name())
            .collect::<Vec<_>>()
            .join(", ");
        let mut query = format!("SELECT {selected_fields} FROM {}", self.object.api_name());

        if !self.filters.is_empty() {
            let filters = self
                .filters
                .iter()
                .map(SalesforceFilter::to_soql)
                .collect::<Result<Vec<_>, _>>()?
                .join(" AND ");
            query.push_str(" WHERE ");
            query.push_str(&filters);
        }

        if !self.order_by.is_empty() {
            let order_by = self
                .order_by
                .iter()
                .map(|sort| sort.to_soql())
                .collect::<Vec<_>>()
                .join(", ");
            query.push_str(" ORDER BY ");
            query.push_str(&order_by);
        }

        if let Some(limit) = self.limit {
            query.push_str(" LIMIT ");
            query.push_str(&limit.to_string());
        }

        Ok(query)
    }

    fn validate(&self) -> Result<(), SalesforceProviderError> {
        if self.selected_fields.is_empty() {
            return Err(SalesforceProviderError::InvalidSoql(
                "at least one selected field is required".to_string(),
            ));
        }

        for field in &self.selected_fields {
            validate_field_object(self.object, *field)?;
        }

        for filter in &self.filters {
            validate_field_object(self.object, filter.field)?;
        }

        for sort in &self.order_by {
            validate_field_object(self.object, sort.field)?;
        }

        if matches!(self.limit, Some(0)) {
            return Err(SalesforceProviderError::InvalidSoql(
                "limit must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }
}

fn validate_field_object(
    object: SalesforceObject,
    field: SalesforceField,
) -> Result<(), SalesforceProviderError> {
    if field.object() != object {
        return Err(SalesforceProviderError::InvalidSoql(format!(
            "{} is not registered on {}",
            field.api_name(),
            object.api_name()
        )));
    }

    Ok(())
}

fn escape_soql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn validate_salesforce_id(value: &str) -> Result<(), SalesforceProviderError> {
    if matches!(value.len(), 15 | 18) && value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Ok(());
    }

    Err(SalesforceProviderError::InvalidParameter {
        name: "id",
        reason: "must be a 15 or 18 character Salesforce id",
    })
}

fn validate_salesforce_ids(values: &[String]) -> Result<(), SalesforceProviderError> {
    if values.is_empty() {
        return Err(SalesforceProviderError::InvalidParameter {
            name: "ids",
            reason: "must include at least one Salesforce id",
        });
    }

    for value in values {
        validate_salesforce_id(value)?;
    }

    Ok(())
}

fn validate_datetime_literal(value: &str) -> Result<(), SalesforceProviderError> {
    let has_basic_shape = value.len() >= 20
        && value.contains('T')
        && (value.ends_with('Z') || value.contains('+') || value.rmatch_indices('-').count() >= 3);
    let has_safe_chars = value
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '-' | ':' | 'T' | 'Z' | '.' | '+'));

    if has_basic_shape && has_safe_chars {
        return Ok(());
    }

    Err(SalesforceProviderError::InvalidParameter {
        name: "datetime",
        reason: "must be an ISO-8601 Salesforce datetime literal",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_account_query_with_registered_fields() {
        let query = SalesforceSoqlQuery::builder(SalesforceObject::Account)
            .select([
                SalesforceField::AccountId,
                SalesforceField::AccountName,
                SalesforceField::AccountSystemModstamp,
            ])
            .filter(
                SalesforceFilter::new(
                    SalesforceField::AccountId,
                    SalesforceOperator::Eq,
                    SalesforceLiteral::Id("001000000000001AAA".to_string()),
                )
                .expect("valid filter"),
            )
            .limit(1)
            .to_soql()
            .expect("query");

        assert_eq!(
            query,
            "SELECT Id, Name, SystemModstamp FROM Account WHERE Id = '001000000000001AAA' LIMIT 1"
        );
    }

    #[test]
    fn rejects_unregistered_operator_for_text_field() {
        let error = SalesforceFilter::new(
            SalesforceField::AccountName,
            SalesforceOperator::Gt,
            SalesforceLiteral::Text("Acme".to_string()),
        )
        .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidSoql(
                "operator > is not registered for Name".to_string()
            )
        );
    }

    #[test]
    fn rejects_injected_salesforce_id() {
        let error = SalesforceFilter::new(
            SalesforceField::AccountId,
            SalesforceOperator::Eq,
            SalesforceLiteral::Id("001000000000001' OR Name != ''".to_string()),
        )
        .unwrap_err();

        assert_eq!(
            error,
            SalesforceProviderError::InvalidParameter {
                name: "id",
                reason: "must be a 15 or 18 character Salesforce id"
            }
        );
    }

    #[test]
    fn rejects_account_id_operator_literal_mismatches() {
        let eq_with_set = SalesforceFilter::new(
            SalesforceField::AccountId,
            SalesforceOperator::Eq,
            SalesforceLiteral::IdSet(vec!["001000000000001AAA".to_string()]),
        )
        .unwrap_err();
        let in_with_scalar = SalesforceFilter::new(
            SalesforceField::AccountId,
            SalesforceOperator::In,
            SalesforceLiteral::Id("001000000000001AAA".to_string()),
        )
        .unwrap_err();

        assert_eq!(
            eq_with_set,
            SalesforceProviderError::InvalidSoql(
                "operator = is not registered with this literal type for Id".to_string()
            )
        );
        assert_eq!(
            in_with_scalar,
            SalesforceProviderError::InvalidSoql(
                "operator IN is not registered with this literal type for Id".to_string()
            )
        );
    }

    #[test]
    fn escapes_text_literals() {
        let query = SalesforceSoqlQuery::builder(SalesforceObject::Account)
            .select([SalesforceField::AccountId])
            .filter(
                SalesforceFilter::new(
                    SalesforceField::AccountName,
                    SalesforceOperator::Eq,
                    SalesforceLiteral::Text("Bob's \\ Team".to_string()),
                )
                .expect("valid filter"),
            )
            .to_soql()
            .expect("query");

        assert_eq!(
            query,
            "SELECT Id FROM Account WHERE Name = 'Bob\\'s \\\\ Team'"
        );
    }
}
