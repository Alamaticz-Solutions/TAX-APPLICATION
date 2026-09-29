
# Admin and CRM operations can manage activity types.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Operational CRM users can read lookup values needed by forms and reports.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "sales_rep", "support_user", "marketing_user"])
    res := {"allow": true, "filter": {}}
}
