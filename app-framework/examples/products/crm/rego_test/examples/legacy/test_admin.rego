package system.office
import rego.v1


# Define the default permission to be false (deny by default)
default allow = false


# Allow full access to admins for all tables
allow if {
    input.roles[_] == "admin"
}
