
# Admin and CRM operations can manage all quote line items.
access := res if {
    check_schema_type()
    has_any_role(input.user, ["admin", "crm_ops"])
    res := {"allow": true, "filter": {}}
}

# Sales managers and finance can inspect all line-item economics.
access := res if {
    check_schema_type()
    input.action == "read"
    has_any_role(input.user, ["sales_manager", "finance_user"])
    res := {"allow": true, "filter": {}}
}

# Sales reps can edit line items that stay within standard discount guardrails.
access := res if {
    check_schema_type()
    input.action in ["create", "read", "update"]
    has_role(input.user, "sales_rep")
    res := {"allow": true, "filter": {"discount": {"_lte": 20.0}}}
}
