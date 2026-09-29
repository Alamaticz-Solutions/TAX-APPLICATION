import { crmUiContract } from "../../generated/appfw-ui-contract";
import { EntityListView } from "../../scaffold/EntityListView";
import { PageHeader, StateView } from "../../components/ui";

// Human-owned (ADR 0009). Customize toward a stage-based pipeline (kanban,
// aggregate summary) over the generic opportunity list.
const opportunity = crmUiContract.entities.find((entity) => entity.routeSegment === "pipeline");

export function PipelineScreen() {
  if (!opportunity) {
    return <StateView kind="error" title="Opportunity entity is missing from the contract" />;
  }
  return (
    <>
      <PageHeader
        title="Pipeline"
        subtitle="Opportunities — sortable list, aggregate summary, pagination, empty/loading/error states"
      />
      <EntityListView entity={opportunity} />
    </>
  );
}
