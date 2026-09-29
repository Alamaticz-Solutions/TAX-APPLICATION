
# Admin and CRM operations can manage all leads.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Marketing can nurture and qualify unconverted leads.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_any_role(input.user, ["marketing_user", "marketing_manager"])
    res := {"allow": true, "filter": {"is_converted": {"_eq": false}}}
}

# Sales managers can review all lead flow.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "sales_manager")
    res := {"allow": true, "filter": {}}
}

# Sales reps can see qualified unconverted leads only.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {
        "_and": [
            {"is_converted": {"_eq": false}},
            {"company": {"_ne": null}}
        ]
    }}
}
