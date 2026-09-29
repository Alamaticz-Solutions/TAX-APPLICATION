# CRM Features

This directory is for human-owned CRM workflow screens, not one folder per entity
type.

The generated UI contract exposes a small set of product workflows such as
Accounts, Pipeline, Activities, and Audit. Those screens may compose multiple
entities, custom methods, charts, and workflow-specific UX.

The full entity catalog is rendered through the generic model-driven scaffold in
`../scaffold`, with routes under `/data/:routeSegment`. Add a new feature folder
only when an entity needs a first-class product workflow that should be more
opinionated than the generic scaffold.
