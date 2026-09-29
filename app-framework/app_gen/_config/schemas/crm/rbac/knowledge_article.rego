# Admin and CRM operations can manage knowledge articles.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Support users can read public published knowledge.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "support_user")
    res := {"allow": true, "filter": {
        "_and": [
            {"is_public": {"_eq": true}},
            {"status": {"_eq": "Published"}}
        ]
    }}
}
