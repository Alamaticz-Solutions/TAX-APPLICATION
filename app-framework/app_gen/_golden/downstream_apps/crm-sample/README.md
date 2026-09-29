# CRM Sample Downstream Template

This golden profile is the default downstream application template used by
`scripts/appfw product new --profile crm-sample`.

The generated app starts from `examples/products/crm`, then receives the profile
overlay and a fresh `appfw.lock`. The result is a split-root product checkout:
product config, generated artifacts, and extension code live in the app while
framework CLI, generator, templates, runtime crates, and test harnesses are
resolved from the selected app-framework checkout.
