use appfw_saas_core::{PageCountContinuation, PaginationContinuation};

use crate::{
    WorkdayProviderError, WorkdayRequestPlan, WorkdayResponseFilter, WorkdayResponsePageSummary,
};

pub type WorkdayPageCountContinuationResult =
    Result<Option<PageCountContinuation>, WorkdayProviderError>;
pub type WorkdayPaginationContinuationResult =
    Result<Option<PaginationContinuation>, WorkdayProviderError>;
pub type WorkdayResponseFilterContinuationResult =
    Result<WorkdayResponseFilter, WorkdayProviderError>;

pub fn next_workday_page_count_continuation(
    plan: &WorkdayRequestPlan,
    response_page: &WorkdayResponsePageSummary,
) -> WorkdayPageCountContinuationResult {
    plan.response_filter.validate()?;
    validate_max_pages(plan.max_pages)?;
    validate_response_page_matches_request(plan.response_filter.page, response_page.page)?;

    if response_page
        .total_pages
        .is_some_and(|total_pages| plan.response_filter.page >= total_pages)
        || plan.response_filter.page >= plan.max_pages
    {
        return Ok(None);
    }

    let stable_as_of = plan.response_filter.as_of_entry_datetime.clone().ok_or(
        WorkdayProviderError::InvalidParameter {
            name: "response_filter.as_of_entry_datetime",
            reason: "must be set when traversing pages after page 1",
        },
    )?;

    let continuation = PageCountContinuation::new(
        plan.response_filter.page.saturating_add(1),
        u32::from(plan.response_filter.count),
    )
    .with_stable_as_of(stable_as_of);

    continuation
        .validate()
        .map_err(WorkdayProviderError::InvalidSaasRequestPlan)?;

    Ok(Some(continuation))
}

pub fn next_workday_pagination_continuation(
    plan: &WorkdayRequestPlan,
    response_page: &WorkdayResponsePageSummary,
) -> WorkdayPaginationContinuationResult {
    next_workday_page_count_continuation(plan, response_page)
        .map(|continuation| continuation.map(PaginationContinuation::PageCount))
}

pub fn response_filter_from_page_count_continuation(
    base_filter: &WorkdayResponseFilter,
    continuation: &PageCountContinuation,
) -> WorkdayResponseFilterContinuationResult {
    base_filter.validate()?;
    continuation
        .validate()
        .map_err(WorkdayProviderError::InvalidSaasRequestPlan)?;

    let stable_as_of =
        continuation
            .stable_as_of
            .clone()
            .ok_or(WorkdayProviderError::InvalidParameter {
                name: "response_filter.as_of_entry_datetime",
                reason: "must be set when traversing pages after page 1",
            })?;

    if continuation.count > u32::from(WorkdayResponseFilter::MAX_COUNT) {
        return Err(WorkdayProviderError::InvalidParameter {
            name: "response_filter.count",
            reason: "must be in the Workday Count range 1..999",
        });
    }

    let mut response_filter = WorkdayResponseFilter::new(
        continuation.page,
        u16::try_from(continuation.count).map_err(|_| WorkdayProviderError::InvalidParameter {
            name: "response_filter.count",
            reason: "must be in the Workday Count range 1..999",
        })?,
    )
    .with_as_of_entry_datetime(stable_as_of);
    response_filter.as_of_effective_date = base_filter.as_of_effective_date.clone();
    response_filter.validate()?;

    Ok(response_filter)
}

impl WorkdayResponsePageSummary {
    pub fn next_page_count_continuation(
        &self,
        plan: &WorkdayRequestPlan,
    ) -> WorkdayPageCountContinuationResult {
        next_workday_page_count_continuation(plan, self)
    }

    pub fn next_pagination_continuation(
        &self,
        plan: &WorkdayRequestPlan,
    ) -> WorkdayPaginationContinuationResult {
        next_workday_pagination_continuation(plan, self)
    }
}

fn validate_max_pages(max_pages: u32) -> Result<(), WorkdayProviderError> {
    if max_pages == 0 {
        return Err(WorkdayProviderError::InvalidParameter {
            name: "limits.max_pages",
            reason: "must be greater than zero",
        });
    }

    Ok(())
}

fn validate_response_page_matches_request(
    request_page: u32,
    response_page: Option<u32>,
) -> Result<(), WorkdayProviderError> {
    if let Some(response_page) = response_page {
        if response_page != request_page {
            return Err(WorkdayProviderError::InvalidResponse(format!(
                "response Page {response_page} does not match requested page {request_page}"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GetLocationsRequestTemplate, WorkdayQueryLimits, WorkdayRequestPlan,
        WorkdayRequestTemplate, WorkdayResponseFilter, WorkdaySoapRequestTemplate,
    };

    const STABLE_AS_OF: &str = "2026-06-26T04:00:00Z";

    #[test]
    fn next_continuation_from_page_one_preserves_count_and_stable_as_of() {
        let plan = request_plan(1, 50, Some(STABLE_AS_OF), 10);
        let response_page = response_page_summary(1, 3);

        let continuation = next_workday_page_count_continuation(&plan, &response_page)
            .expect("next page")
            .expect("continuation");

        assert_eq!(
            continuation,
            PageCountContinuation::new(2, 50).with_stable_as_of(STABLE_AS_OF)
        );
        assert_eq!(
            response_page
                .next_pagination_continuation(&plan)
                .expect("shared continuation"),
            Some(PaginationContinuation::PageCount(continuation))
        );
    }

    #[test]
    fn stops_at_response_total_pages() {
        let plan = request_plan(3, 50, Some(STABLE_AS_OF), 10);
        let response_page = response_page_summary(3, 3);

        assert_eq!(
            next_workday_page_count_continuation(&plan, &response_page).expect("stop"),
            None
        );
    }

    #[test]
    fn stops_at_plan_max_pages() {
        let plan = request_plan(2, 50, Some(STABLE_AS_OF), 2);
        let response_page = response_page_summary(2, 10);

        assert_eq!(
            next_workday_page_count_continuation(&plan, &response_page).expect("stop"),
            None
        );
    }

    #[test]
    fn rejects_next_page_without_stable_as_of() {
        let plan = request_plan(1, 50, None, 10);
        let response_page = response_page_summary(1, 2);

        assert_eq!(
            next_workday_page_count_continuation(&plan, &response_page).unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "response_filter.as_of_entry_datetime",
                reason: "must be set when traversing pages after page 1",
            }
        );
    }

    #[test]
    fn rejects_response_page_mismatch() {
        let plan = request_plan(1, 50, Some(STABLE_AS_OF), 10);
        let response_page = response_page_summary(2, 3);

        assert_eq!(
            next_workday_page_count_continuation(&plan, &response_page).unwrap_err(),
            WorkdayProviderError::InvalidResponse(
                "response Page 2 does not match requested page 1".to_string()
            )
        );
    }

    #[test]
    fn continuation_response_filter_preserves_effective_date_and_stable_entry_datetime() {
        let base_filter = WorkdayResponseFilter::new(1, 50)
            .with_as_of_effective_date("2026-06-25")
            .with_as_of_entry_datetime(STABLE_AS_OF);
        let continuation = PageCountContinuation::new(2, 50).with_stable_as_of(STABLE_AS_OF);

        let response_filter =
            response_filter_from_page_count_continuation(&base_filter, &continuation)
                .expect("response filter");

        assert_eq!(
            response_filter,
            WorkdayResponseFilter::new(2, 50)
                .with_as_of_effective_date("2026-06-25")
                .with_as_of_entry_datetime(STABLE_AS_OF)
        );
    }

    #[test]
    fn continuation_response_filter_rejects_next_page_without_stable_as_of() {
        let base_filter = WorkdayResponseFilter::new(1, 50).with_as_of_effective_date("2026-06-25");
        let continuation = PageCountContinuation::new(2, 50);

        assert_eq!(
            response_filter_from_page_count_continuation(&base_filter, &continuation).unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "response_filter.as_of_entry_datetime",
                reason: "must be set when traversing pages after page 1",
            }
        );
    }

    #[test]
    fn continuation_response_filter_rejects_count_above_workday_max() {
        let base_filter = WorkdayResponseFilter::new(1, 50).with_as_of_entry_datetime(STABLE_AS_OF);
        let continuation =
            PageCountContinuation::new(2, u32::from(WorkdayResponseFilter::MAX_COUNT) + 1)
                .with_stable_as_of(STABLE_AS_OF);

        assert_eq!(
            response_filter_from_page_count_continuation(&base_filter, &continuation).unwrap_err(),
            WorkdayProviderError::InvalidParameter {
                name: "response_filter.count",
                reason: "must be in the Workday Count range 1..999",
            }
        );
    }

    fn request_plan(
        page: u32,
        count: u16,
        stable_as_of: Option<&str>,
        max_pages: u32,
    ) -> WorkdayRequestPlan {
        let response_filter = stable_as_of
            .map(|value| WorkdayResponseFilter::new(page, count).with_as_of_entry_datetime(value))
            .unwrap_or_else(|| WorkdayResponseFilter::new(page, count));
        let request_template = WorkdayRequestTemplate::GetLocations(GetLocationsRequestTemplate {
            include_inactive: false,
            response_filter: response_filter.clone(),
        });
        let soap_envelope = request_template.soap_envelope().expect("soap envelope");

        WorkdayRequestPlan::new(
            "Get_Locations",
            request_template,
            soap_envelope,
            response_filter,
            WorkdayQueryLimits {
                timeout_ms: WorkdayQueryLimits::DEFAULT_TIMEOUT_MS,
                max_pages,
            },
        )
    }

    fn response_page_summary(page: u32, total_pages: u32) -> WorkdayResponsePageSummary {
        WorkdayResponsePageSummary {
            total_results: None,
            total_pages: Some(total_pages),
            page_results: None,
            page: Some(page),
        }
    }
}
