import { crmUiContract } from "../../generated/appfw-ui-contract";
import { EntityListView } from "../../scaffold/EntityListView";
import { PageHeader, StateView } from "../../components/ui";

// Human-owned (ADR 0009). Starts from the generic list; customize toward an
// account 360 (relationships, activity timeline) without touching the scaffold.
const account = crmUiContract.entities.find((entity) => entity.routeSegment === "accounts");

export function AccountsScreen() {
  if (!account) {
    return <StateView kind="error" title="Account entity is missing from the contract" />;
  }
  return (
    <>
      <PageHeader
        title="Accounts"
        subtitle="Customer accounts — relationship navigation, tenant filtering, field validation"
      />
      <EntityListView entity={account} />
    </>
  );
}
