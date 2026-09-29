package system.permissions


# Define the default permission to be false (deny by default)
default allow = false


# Allow read access to public tables for all authenticated users
allow {
    input.action == "read"
    input.table == "public_info"
    input.user.authenticated == true
}
