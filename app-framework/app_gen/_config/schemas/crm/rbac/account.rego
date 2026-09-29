# Admin and CRM operations can manage all accounts.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales managers can work the full account book but cannot delete records.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_role(input.user, "sales_manager")
    res := {"allow": true, "filter": {}}
}

# Field sales users are region-scoped to the western US.
access := res if {
    check_schema_type()
    input.action in ["read", "update"]
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {
        "_and": [
            {"billing_country": {"_eq": "USA"}},
            {"billing_state": {"_in": ["CA", "OR", "WA", "NV", "AZ"]}}
        ]
    }}
}

# Finance can inspect customer account context but does not mutate accounts.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "finance_user")
    res := {"allow": true, "filter": {"billing_country": {"_eq": "USA"}}}
}

# Provider certification fixture: proves policy filters cannot be widened.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "account_ca_reader")
    res := {"allow": true, "filter": {"billing_state": {"_eq": "CA"}}}
}
