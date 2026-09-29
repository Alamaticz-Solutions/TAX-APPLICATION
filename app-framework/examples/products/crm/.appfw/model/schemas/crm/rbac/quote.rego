
# Admin and CRM operations can manage all quotes.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales managers and finance can review all quotes.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "finance_user"])
    res := {"allow": true, "filter": {}}
}

# Sales reps can create and update ordinary quotes below approval thresholds.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {
        "_and": [
            {"discount": {"_lte": 25.0}},
            {"total_price": {"_lte": 100000.0}}
        ]
    }}
}
