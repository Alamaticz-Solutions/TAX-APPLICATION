
# Admin and CRM operations can manage all opportunities.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales managers can run the full opportunity pipeline.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_role(input.user, "sales_manager")
    res := {"allow": true, "filter": {}}
}

# Sales reps work ordinary pipeline deals, with larger strategic deals escalated.
access := res if {
    check_schema_type()
    input.action in ["read", "update"]
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {
        "_and": [
            {"amount": {"_lte": 250000.0}},
            {"lead_source": {"_ne": "Executive Referral"}}
        ]
    }}
}

# Finance can review larger commercial exposure and upcoming closes.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "finance_user")
    res := {"allow": true, "filter": {"amount": {"_gte": 50000.0}}}
}
