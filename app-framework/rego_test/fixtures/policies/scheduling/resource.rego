package scheduling.resource
import rego.v1

# Fixture policy that mirrors the backend contract:
# data.<schema>.<entity>.access returns {"allow": boolean, "filter"?: object}.

pds_tenant_id := "180000"

default access = {"allow": false}

read_roles := {
    "territory_manager",
    "regional_manager",
    "operations_manager",
    "dental_assistant",
    "clinical_contractor",
}

mutate_roles := {
    "regional_manager",
    "operations_manager",
}

mutate_actions := {
    "create",
    "update",
    "delete",
}

valid_resource_request if {
    input.schema_name == "scheduling"
    input.entity_type == "resource"
}

is_admin if {
    has_role(input.user, "admin")
}

# Admins bypass tenant scoping because they can operate across all tenants.
access := {"allow": true, "filter": {}} if {
    valid_resource_request
    is_admin
}

access := {"allow": true, "filter": tenant_filter(input.user)} if {
    valid_resource_request
    not is_admin
    input.action == "read"
    has_any_role(input.user, read_roles)
}

access := {"allow": true, "filter": tenant_filter(input.user)} if {
    valid_resource_request
    not is_admin
    mutate_actions[input.action]
    has_any_role(input.user, mutate_roles)
}

has_any_role(user, allowed_roles) if {
    some role in user.roles
    allowed_roles[role]
}

has_role(user, role) if {
    user.roles[_] == role
}

tenant_filter(user) := filter if {
    # PDS Health users are not tenant-scoped.
    user.tenant_id == pds_tenant_id
    filter := {}
} else := filter if {
    filter := {"tenant_id": user.tenant_id}
}
