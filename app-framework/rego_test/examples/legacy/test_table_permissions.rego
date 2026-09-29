package system.office
import rego.v1


# Define the default permission to be false (deny by default)
default allow = false



# Allow full access to admins to table
allow if {
    input.user.roles[_] == "admin"
}

# office: read
allow if {
    input.table == "office"
    input.action == "read"
    has_any_role(input.user, ["roc_admin", "dev_ops"])
}

# office: write
allow if {
    input.table == "office"
    input.action == "write"
    has_any_role(input.user, ["roc_admin", "system_admin"])
}




# Helper rule to check if a user has any of the required roles
has_any_role(user, required_roles) if {
    some role in required_roles
    has_role(user, role)
}

# Helper rule to check if a user has a specific role
has_role(user, role) if {
    user.roles[_] == role
}