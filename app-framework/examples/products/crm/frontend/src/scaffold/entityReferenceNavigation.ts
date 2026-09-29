import type {
  AppfwUiContract,
  AppfwUiEntityContract,
  AppfwUiViewRegistryContract,
  AppfwUiWorkflowContract
} from "../generated/appfw-ui-contract";

export type AnswerEnvelopeEntityRef = {
  kind?: string;
  record_locator?: string;
  view_hint?: string;
  entity?: {
    schemaName?: string;
    typeName?: string;
  };
  ids?: readonly string[];
  idKind?: string;
  query?: {
    filter?: unknown;
  };
};

export type EntityReferenceNavigation = {
  entity: AppfwUiEntityContract;
  idKind: string;
  recordLocator: string | null;
  detailHref: string | null;
  listHref: string;
  filteredListHref: string | null;
  nativeView: AppfwUiViewRegistryContract | null;
  timelineWorkflow: AppfwUiWorkflowContract | null;
  resolveOperation: {
    graphqlName: string;
    variableName: "locator";
    variables: { locator: string };
  } | null;
  exposesPrimaryKey: false;
};

export function composeEntityReferenceNavigation(
  contract: AppfwUiContract,
  ref: AnswerEnvelopeEntityRef
): EntityReferenceNavigation | null {
  const entity = findReferencedEntity(contract, ref);
  if (!entity) return null;

  const routeKind = entity.addressing.idKinds.find((kind) => kind.kind === entity.addressing.routeIdKind);
  if (!routeKind?.answerEnvelope || !routeKind.routeSafe || routeKind.kind !== "record_locator") {
    return null;
  }

  const recordLocator = recordLocatorFromRef(ref);
  const viewHint = normalizeViewHint(ref.view_hint);
  const nativeView = findNativeView(contract, entity, viewHint, Boolean(recordLocator));
  const filteredListHref = ref.query?.filter
    ? entity.addressing.filteredListRoute.replace(":filterJson", encodeURIComponent(JSON.stringify(ref.query.filter)))
    : null;
  const locatorOperation = recordLocator ? entity.addressing.locatorResolveOperation : undefined;

  return {
    entity,
    idKind: routeKind.kind,
    recordLocator,
    detailHref: recordLocator ? entity.addressing.routeTemplate.replace(":recordLocator", recordLocator) : null,
    listHref: entity.addressing.listRoute,
    filteredListHref,
    nativeView,
    timelineWorkflow: findTimelineWorkflow(contract, entity),
    resolveOperation: recordLocator && locatorOperation
      ? {
          graphqlName: locatorOperation.graphqlName,
          variableName: "locator",
          variables: { locator: recordLocator }
        }
      : null,
    exposesPrimaryKey: false
  };
}

function findReferencedEntity(contract: AppfwUiContract, ref: AnswerEnvelopeEntityRef) {
  return contract.entities.find((entity) => {
    const schemaMatches = !ref.entity?.schemaName || ref.entity.schemaName === entity.schemaName;
    const typeMatches = ref.entity?.typeName
      ? ref.entity.typeName === entity.typeName
      : ref.kind === entity.typeName || entity.addressing.entityUri === ref.kind;
    return schemaMatches && typeMatches;
  });
}

function recordLocatorFromRef(ref: AnswerEnvelopeEntityRef) {
  if (typeof ref.record_locator === "string" && ref.record_locator.startsWith("rl_")) {
    return ref.record_locator;
  }
  if (ref.idKind === "record_locator") {
    return ref.ids?.find((id) => id.startsWith("rl_")) ?? null;
  }
  return null;
}

function findNativeView(
  contract: AppfwUiContract,
  entity: AppfwUiEntityContract,
  viewHint: string,
  hasRecordLocator: boolean
) {
  const preferredShape = viewHint === "detail" && hasRecordLocator ? "record_detail" : viewHint;
  return contract.viewRegistry.find((view) =>
    view.supports.entityTypes.includes(entity.typeName) &&
    view.supports.idKinds.includes("record_locator") &&
    view.supports.shapes.includes(preferredShape)
  ) ?? null;
}

function findTimelineWorkflow(contract: AppfwUiContract, entity: AppfwUiEntityContract) {
  return contract.workflows.find((workflow) =>
    workflow.proofPoints.includes("read_only_timeline") &&
    (workflow.primaryEntity === entity.typeName || workflow.supportingEntities.includes(entity.typeName))
  ) ?? null;
}

function normalizeViewHint(viewHint: string | undefined) {
  if (!viewHint || viewHint === "list") return "filtered_list";
  if (viewHint === "flow-graph") return "flow_graph";
  if (viewHint === "timeline") return "timeline";
  return viewHint;
}
