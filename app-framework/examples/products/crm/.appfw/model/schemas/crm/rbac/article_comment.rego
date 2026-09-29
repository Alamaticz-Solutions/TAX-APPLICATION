# Admin and CRM operations can manage article comments.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Support users can read external article feedback.
access := res if {
    check_schema_type()
    input.action == "read"
    has_role(input.user, "support_user")
    res := {"allow": true, "filter": {"is_internal": {"_eq": false}}}
}
