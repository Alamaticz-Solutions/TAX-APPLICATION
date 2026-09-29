use chrono::NaiveDate;
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    product_api::{DataAccess, EntityType, HandlerResult, JsonValue, UserAuth},
    schemas::crm::{
        AccountProjection, ActivityProjection, ContactProjection, OpportunityProjection,
        QuoteProjection,
    },
};

const HEALTH_SAMPLE_LIMIT: i32 = 250;

pub(crate) async fn summarize_account_health(
    user: Option<UserAuth>,
    data_access: &Arc<DataAccess>,
    entity_type: &Arc<EntityType>,
    account_id: String,
) -> HandlerResult<JsonValue> {
    let today = chrono::Utc::now().date_naive();
    let account = data_access
        .find_item::<AccountProjection>(
            entity_type.clone(),
            selection(
                "account",
                &[
                    field("id"),
                    field("name"),
                    field("website"),
                    field("phone"),
                    field("email"),
                    field("annual_revenue"),
                    field("health_score"),
                    field("health_last_refreshed_at"),
                    field("number_of_employees"),
                    field("billing_city"),
                    field("billing_state"),
                    field("billing_country"),
                    field("industry_id"),
                    field("version"),
                ],
            ),
            account_id.clone(),
            user.clone(),
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("account `{account_id}` was not found"))?;

    let opportunities_type = data_access
        .app_config
        .get_entity_type(&"crm".to_string(), &"Opportunity".to_string())?;
    let activities_type = data_access
        .app_config
        .get_entity_type(&"crm".to_string(), &"Activity".to_string())?;
    let quotes_type = data_access
        .app_config
        .get_entity_type(&"crm".to_string(), &"Quote".to_string())?;
    let contacts_type = data_access
        .app_config
        .get_entity_type(&"crm".to_string(), &"Contact".to_string())?;

    let opportunities = data_access
        .query_items::<OpportunityProjection>(
            opportunities_type,
            selection(
                "opportunities",
                &[
                    field("id"),
                    field("name"),
                    field("amount"),
                    field("close_date"),
                    field("account_id"),
                    field("stage_id"),
                    nested(
                        "stage",
                        &[
                            field("id"),
                            field("name"),
                            field("probability"),
                            field("is_closed"),
                            field("is_won"),
                            field("sort_order"),
                        ],
                    ),
                ],
            ),
            Some(json!({ "account_id": { "_eq": account_id } })),
            None,
            0,
            HEALTH_SAMPLE_LIMIT,
            None,
            user.clone(),
        )
        .await?;

    let activities = data_access
        .query_items::<ActivityProjection>(
            activities_type,
            selection(
                "activities",
                &[
                    field("id"),
                    field("subject"),
                    field("activity_date"),
                    field("due_date"),
                    field("is_closed"),
                    field("priority"),
                    field("status"),
                    field("account_id"),
                ],
            ),
            Some(json!({ "account_id": { "_eq": account_id } })),
            None,
            0,
            HEALTH_SAMPLE_LIMIT,
            None,
            user.clone(),
        )
        .await?;

    let quotes = data_access
        .query_items::<QuoteProjection>(
            quotes_type,
            selection(
                "quotes",
                &[
                    field("id"),
                    field("name"),
                    field("quote_number"),
                    field("expiration_date"),
                    field("subtotal"),
                    field("discount"),
                    field("tax"),
                    field("total_price"),
                    field("account_id"),
                    field("status_id"),
                    nested("status", &[field("id"), field("name"), field("sort_order")]),
                ],
            ),
            Some(json!({ "account_id": { "_eq": account_id } })),
            None,
            0,
            HEALTH_SAMPLE_LIMIT,
            None,
            user.clone(),
        )
        .await?;

    let contacts = data_access
        .query_items::<ContactProjection>(
            contacts_type,
            selection(
                "contacts",
                &[
                    field("id"),
                    field("first_name"),
                    field("last_name"),
                    field("email"),
                    field("phone"),
                    field("mobile"),
                    field("title"),
                    field("department"),
                    field("account_id"),
                ],
            ),
            Some(json!({ "account_id": { "_eq": account_id } })),
            None,
            0,
            HEALTH_SAMPLE_LIMIT,
            None,
            user,
        )
        .await?;

    Ok(account_health_summary(
        account,
        opportunities.items,
        opportunities.query_count,
        activities.items,
        activities.query_count,
        quotes.items,
        quotes.query_count,
        contacts.items,
        contacts.query_count,
        today,
    ))
}

fn account_health_summary(
    account: AccountProjection,
    opportunities: Vec<OpportunityProjection>,
    opportunity_count: i64,
    activities: Vec<ActivityProjection>,
    activity_count: i64,
    quotes: Vec<QuoteProjection>,
    quote_count: i64,
    contacts: Vec<ContactProjection>,
    contact_count: i64,
    today: NaiveDate,
) -> JsonValue {
    let open_opportunities = opportunities
        .iter()
        .filter(|opportunity| is_open_opportunity(opportunity))
        .collect::<Vec<_>>();
    let won_opportunity_count = opportunities
        .iter()
        .filter(|opportunity| {
            opportunity
                .stage
                .as_ref()
                .and_then(|stage| stage.is_won)
                .unwrap_or(false)
        })
        .count();
    let closed_lost_opportunity_count = opportunities
        .iter()
        .filter(|opportunity| {
            opportunity
                .stage
                .as_ref()
                .and_then(|stage| stage.is_closed)
                .unwrap_or(false)
                && !opportunity
                    .stage
                    .as_ref()
                    .and_then(|stage| stage.is_won)
                    .unwrap_or(false)
        })
        .count();
    let pipeline_value = open_opportunities
        .iter()
        .filter_map(|opportunity| opportunity.amount)
        .sum::<f64>();
    let weighted_pipeline_value = open_opportunities
        .iter()
        .map(|opportunity| {
            let probability = opportunity
                .stage
                .as_ref()
                .and_then(|stage| stage.probability)
                .map(f64::from)
                .unwrap_or(25.0)
                / 100.0;
            opportunity.amount.unwrap_or_default() * probability
        })
        .sum::<f64>();

    let open_activity_count = activities
        .iter()
        .filter(|activity| !activity.is_closed.unwrap_or(false))
        .count();
    let overdue_activity_count = activities
        .iter()
        .filter(|activity| {
            !activity.is_closed.unwrap_or(false)
                && activity.due_date.map(|date| date < today).unwrap_or(false)
        })
        .count();
    let last_activity_date = activities.iter().filter_map(activity_reference_date).max();
    let next_due_date = activities
        .iter()
        .filter(|activity| !activity.is_closed.unwrap_or(false))
        .filter_map(|activity| activity.due_date)
        .filter(|date| *date >= today)
        .min();
    let stale_days = last_activity_date.map(|date| today.signed_duration_since(date).num_days());

    let quote_value = quotes
        .iter()
        .filter_map(|quote| quote.total_price)
        .sum::<f64>();
    let open_quotes = quotes
        .iter()
        .filter(|quote| is_open_quote(quote, today))
        .collect::<Vec<_>>();
    let open_quote_value = open_quotes
        .iter()
        .filter_map(|quote| quote.total_price)
        .sum::<f64>();
    let expired_quote_count = quotes
        .iter()
        .filter(|quote| {
            quote
                .expiration_date
                .map(|date| date < today)
                .unwrap_or(false)
        })
        .count();

    let complete_contact_count = contacts
        .iter()
        .filter(|contact| {
            has_text(contact.email.as_deref())
                && (has_text(contact.phone.as_deref()) || has_text(contact.mobile.as_deref()))
        })
        .count();
    let missing_email_count = contacts
        .iter()
        .filter(|contact| !has_text(contact.email.as_deref()))
        .count();

    let data_quality_fields = [
        ("website", has_text(account.website.as_deref())),
        ("phone", has_text(account.phone.as_deref())),
        ("email", has_text(account.email.as_deref())),
        (
            "billing_country",
            has_text(account.billing_country.as_deref()),
        ),
        ("industry_id", has_text(account.industry_id.as_deref())),
    ];
    let complete_profile_fields = data_quality_fields
        .iter()
        .filter(|(_, complete)| *complete)
        .count();
    let completeness = (complete_profile_fields as f64 / data_quality_fields.len() as f64) * 100.0;

    let mut signals = Vec::new();
    if contact_count == 0 {
        signals.push(signal(
            "relationship",
            "high",
            "No contacts",
            "No contacts are associated with this account.",
        ));
    }
    if open_opportunities.is_empty() {
        signals.push(signal(
            "pipeline",
            "medium",
            "No active pipeline",
            "No open opportunities were found for this account.",
        ));
    }
    if overdue_activity_count > 0 {
        signals.push(signal(
            "engagement",
            if overdue_activity_count >= 3 {
                "high"
            } else {
                "medium"
            },
            "Overdue activity",
            format!(
                "{overdue_activity_count} open activit{} overdue.",
                if overdue_activity_count == 1 {
                    "y is"
                } else {
                    "ies are"
                }
            ),
        ));
    }
    if stale_days.unwrap_or(i64::MAX) > 90 {
        signals.push(signal(
            "engagement",
            "high",
            "Stale engagement",
            "No activity has been captured in more than 90 days.",
        ));
    } else if stale_days.unwrap_or(i64::MAX) > 30 {
        signals.push(signal(
            "engagement",
            "medium",
            "Aging engagement",
            "No activity has been captured in more than 30 days.",
        ));
    }
    if expired_quote_count > 0 {
        signals.push(signal(
            "quote",
            "medium",
            "Expired quotes",
            format!(
                "{expired_quote_count} quote{} expired.",
                if expired_quote_count == 1 {
                    " has"
                } else {
                    "s have"
                }
            ),
        ));
    }
    if completeness < 80.0 {
        signals.push(signal(
            "data_quality",
            "low",
            "Incomplete account profile",
            format!("Profile completeness is {:.0}%.", completeness),
        ));
    }

    let score = health_score(
        &account,
        contact_count,
        open_opportunities.len(),
        weighted_pipeline_value,
        overdue_activity_count,
        stale_days,
        expired_quote_count,
        completeness,
    );
    let risk_level = risk_level(score, &signals);

    json!({
        "account": {
            "id": account.id,
            "name": account.name,
            "website": account.website,
            "annual_revenue": account.annual_revenue,
            "number_of_employees": account.number_of_employees,
            "location": {
                "city": account.billing_city,
                "state": account.billing_state,
                "country": account.billing_country,
            },
            "version": account.version,
        },
        "health": {
            "score": score,
            "grade": health_grade(score),
            "risk_level": risk_level,
            "summary": health_summary(score, risk_level, open_opportunities.len(), overdue_activity_count, stale_days),
            "generated_at": chrono::Utc::now().to_rfc3339(),
        },
        "stored_snapshot": {
            "health_score": account.health_score,
            "refreshed_at": account.health_last_refreshed_at.map(|dt| dt.to_rfc3339()),
            "source": "crm.refresh_account_health_stored_procedure",
        },
        "financials": {
            "annual_revenue": account.annual_revenue,
            "pipeline_value": round_money(pipeline_value),
            "weighted_pipeline_value": round_money(weighted_pipeline_value),
            "quote_value": round_money(quote_value),
            "open_quote_value": round_money(open_quote_value),
        },
        "opportunities": {
            "total_count": opportunity_count,
            "sampled_count": opportunities.len(),
            "open_count": open_opportunities.len(),
            "won_count": won_opportunity_count,
            "closed_lost_count": closed_lost_opportunity_count,
            "stage_distribution": stage_distribution(&opportunities),
            "top_open": top_open_opportunities(&open_opportunities),
        },
        "quotes": {
            "total_count": quote_count,
            "sampled_count": quotes.len(),
            "open_count": open_quotes.len(),
            "expired_count": expired_quote_count,
            "largest_open": largest_quote(&open_quotes),
        },
        "activities": {
            "total_count": activity_count,
            "sampled_count": activities.len(),
            "open_count": open_activity_count,
            "overdue_count": overdue_activity_count,
            "last_activity_date": last_activity_date.map(|date| date.to_string()),
            "next_due_date": next_due_date.map(|date| date.to_string()),
            "stale_days": stale_days,
            "engagement_status": engagement_status(stale_days, overdue_activity_count),
        },
        "contacts": {
            "total_count": contact_count,
            "sampled_count": contacts.len(),
            "complete_contact_count": complete_contact_count,
            "missing_email_count": missing_email_count,
        },
        "data_quality": {
            "profile_completeness": round_percent(completeness),
            "fields": data_quality_fields
                .iter()
                .map(|(name, complete)| json!({ "name": name, "complete": complete }))
                .collect::<Vec<_>>(),
        },
        "signals": signals,
        "recommended_actions": recommended_actions(overdue_activity_count, stale_days, open_opportunities.len(), contact_count, completeness),
        "analysis": {
            "sample_limit": HEALTH_SAMPLE_LIMIT,
            "note": "Counts reflect policy-scoped data. Detail metrics are computed from the sampled records returned by the provider."
        }
    })
}

fn field(name: &str) -> JsonValue {
    json!({ "name": name, "selection_set": [] })
}

fn nested(name: &str, fields: &[JsonValue]) -> JsonValue {
    json!({ "name": name, "selection_set": fields })
}

fn selection(name: &str, fields: &[JsonValue]) -> JsonValue {
    json!({ "name": name, "selection_set": fields })
}

fn has_text(value: Option<&str>) -> bool {
    value.map(|text| !text.trim().is_empty()).unwrap_or(false)
}

fn activity_reference_date(activity: &ActivityProjection) -> Option<NaiveDate> {
    activity.activity_date.or(activity.due_date)
}

fn is_open_opportunity(opportunity: &OpportunityProjection) -> bool {
    !opportunity
        .stage
        .as_ref()
        .and_then(|stage| stage.is_closed)
        .unwrap_or(false)
}

fn is_open_quote(quote: &QuoteProjection, today: NaiveDate) -> bool {
    let expired = quote
        .expiration_date
        .map(|date| date < today)
        .unwrap_or(false);
    !expired && !quote_status_is_closed(quote)
}

fn quote_status_is_closed(quote: &QuoteProjection) -> bool {
    let Some(status) = quote
        .status
        .as_ref()
        .and_then(|status| status.name.as_deref())
    else {
        return false;
    };
    let status = status.to_lowercase();
    [
        "accepted",
        "approved",
        "closed",
        "won",
        "rejected",
        "cancelled",
        "canceled",
        "expired",
    ]
    .iter()
    .any(|needle| status.contains(needle))
}

fn health_score(
    account: &AccountProjection,
    contact_count: i64,
    open_opportunity_count: usize,
    weighted_pipeline_value: f64,
    overdue_activity_count: usize,
    stale_days: Option<i64>,
    expired_quote_count: usize,
    completeness: f64,
) -> i32 {
    let mut score = 70;
    if account.annual_revenue.unwrap_or_default() > 0.0 {
        score += 6;
    }
    if contact_count > 0 {
        score += 6;
    } else {
        score -= 14;
    }
    if open_opportunity_count > 0 {
        score += 8;
    } else {
        score -= 8;
    }
    if weighted_pipeline_value > 0.0 {
        score += 6;
    }
    match stale_days {
        Some(days) if days <= 30 => score += 8,
        Some(days) if days <= 90 => score += 3,
        Some(_) => score -= 10,
        None => score -= 12,
    }
    score -= (overdue_activity_count.min(4) as i32) * 5;
    score -= (expired_quote_count.min(3) as i32) * 3;
    if completeness >= 90.0 {
        score += 4;
    } else if completeness < 60.0 {
        score -= 8;
    }
    score.clamp(0, 100)
}

fn health_grade(score: i32) -> &'static str {
    match score {
        85..=100 => "A",
        70..=84 => "B",
        55..=69 => "C",
        40..=54 => "D",
        _ => "F",
    }
}

fn risk_level(score: i32, signals: &[JsonValue]) -> &'static str {
    let has_high_signal = signals
        .iter()
        .any(|signal| signal.get("severity").and_then(JsonValue::as_str) == Some("high"));
    if score >= 75 && !has_high_signal {
        "low"
    } else if score >= 55 {
        "medium"
    } else {
        "high"
    }
}

fn health_summary(
    score: i32,
    risk_level: &str,
    open_opportunity_count: usize,
    overdue_activity_count: usize,
    stale_days: Option<i64>,
) -> String {
    let engagement = match stale_days {
        Some(days) if days <= 30 => "recent engagement",
        Some(days) if days <= 90 => "aging engagement",
        Some(_) => "stale engagement",
        None => "no recorded engagement",
    };
    format!(
        "Account health is {risk_level} risk with score {score}. There are {open_opportunity_count} open opportunities, {overdue_activity_count} overdue activities, and {engagement}."
    )
}

fn engagement_status(stale_days: Option<i64>, overdue_count: usize) -> &'static str {
    if overdue_count > 0 {
        "attention_needed"
    } else {
        match stale_days {
            Some(days) if days <= 30 => "healthy",
            Some(days) if days <= 90 => "watch",
            _ => "stale",
        }
    }
}

fn signal(
    category: impl Into<String>,
    severity: impl Into<String>,
    label: impl Into<String>,
    detail: impl Into<String>,
) -> JsonValue {
    json!({
        "category": category.into(),
        "severity": severity.into(),
        "label": label.into(),
        "detail": detail.into(),
    })
}

fn stage_distribution(opportunities: &[OpportunityProjection]) -> Vec<JsonValue> {
    let mut stages = BTreeMap::<String, (usize, f64)>::new();
    for opportunity in opportunities {
        let stage_name = opportunity
            .stage
            .as_ref()
            .and_then(|stage| stage.name.clone())
            .unwrap_or_else(|| "Unstaged".to_string());
        let entry = stages.entry(stage_name).or_insert((0, 0.0));
        entry.0 += 1;
        entry.1 += opportunity.amount.unwrap_or_default();
    }
    stages
        .into_iter()
        .map(|(stage, (count, amount))| {
            json!({
                "stage": stage,
                "count": count,
                "amount": round_money(amount),
            })
        })
        .collect()
}

fn top_open_opportunities(opportunities: &[&OpportunityProjection]) -> Vec<JsonValue> {
    let mut opportunities = opportunities.to_vec();
    opportunities.sort_by(|left, right| {
        right
            .amount
            .unwrap_or_default()
            .partial_cmp(&left.amount.unwrap_or_default())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    opportunities
        .into_iter()
        .take(5)
        .map(|opportunity| {
            json!({
                "id": opportunity.id,
                "name": opportunity.name,
                "amount": opportunity.amount.map(round_money),
                "close_date": opportunity.close_date.map(|date| date.to_string()),
                "stage": opportunity.stage.as_ref().and_then(|stage| stage.name.clone()),
                "probability": opportunity.stage.as_ref().and_then(|stage| stage.probability).map(f64::from).map(round_percent),
            })
        })
        .collect()
}

fn largest_quote(quotes: &[&QuoteProjection]) -> Option<JsonValue> {
    let mut quotes = quotes.to_vec();
    quotes.sort_by(|left, right| {
        right
            .total_price
            .unwrap_or_default()
            .partial_cmp(&left.total_price.unwrap_or_default())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    quotes.first().map(|quote| {
        json!({
            "id": quote.id,
            "name": quote.name,
            "quote_number": quote.quote_number,
            "total_price": quote.total_price.map(round_money),
            "expiration_date": quote.expiration_date.map(|date| date.to_string()),
            "status": quote.status.as_ref().and_then(|status| status.name.clone()),
        })
    })
}

fn recommended_actions(
    overdue_activity_count: usize,
    stale_days: Option<i64>,
    open_opportunity_count: usize,
    contact_count: i64,
    completeness: f64,
) -> Vec<JsonValue> {
    let mut actions = Vec::new();
    if overdue_activity_count > 0 {
        actions.push(json!({
            "priority": "high",
            "label": "Resolve overdue activities",
            "detail": "Review open tasks and close or reschedule overdue work.",
        }));
    }
    if stale_days.unwrap_or(i64::MAX) > 30 {
        actions.push(json!({
            "priority": "medium",
            "label": "Schedule an account touchpoint",
            "detail": "Create a follow-up activity to refresh engagement.",
        }));
    }
    if open_opportunity_count == 0 {
        actions.push(json!({
            "priority": "medium",
            "label": "Review pipeline coverage",
            "detail": "No active opportunities are currently associated with the account.",
        }));
    }
    if contact_count == 0 {
        actions.push(json!({
            "priority": "high",
            "label": "Add stakeholder contacts",
            "detail": "Associate decision makers or operational contacts with the account.",
        }));
    }
    if completeness < 80.0 {
        actions.push(json!({
            "priority": "low",
            "label": "Complete account profile",
            "detail": "Fill missing account profile fields to improve reporting and routing.",
        }));
    }
    if actions.is_empty() {
        actions.push(json!({
            "priority": "low",
            "label": "Maintain current cadence",
            "detail": "No urgent health actions were detected from the sampled account data.",
        }));
    }
    actions
}

fn round_money(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

fn round_percent(value: f64) -> f64 {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}
