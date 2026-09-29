
# Admin and CRM operations can manage all contacts.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales managers can work all contacts.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_role(input.user, "sales_manager")
    res := {"allow": true, "filter": {}}
}

# Sales reps work contacts in their operating territory.
access := res if {
    check_schema_type()
    input.action in ["read", "update"]
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {
        "_or": [
            {"mailing_state": {"_in": ["CA", "OR", "WA", "NV", "AZ"]}},
            {"mailing_country": {"_eq": "USA"}}
        ]
    }}
}

# Support users can read operational contacts.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "support_user")
    res := {"allow": true, "filter": {
        "department": {"_in": ["IT", "Operations", "Customer Success", "Support"]}
    }}
}
