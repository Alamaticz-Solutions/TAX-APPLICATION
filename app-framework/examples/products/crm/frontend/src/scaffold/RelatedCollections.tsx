import { useEffect, useMemo, useState } from "react";
import type { AppfwUiEntityContract, AppfwUiFieldContract } from "../generated/appfw-ui-contract";
import { useAppfwClient, useAuth, useTenant } from "../app/providers";
import { Badge, DataTable } from "../components/ui";
import type { AppfwEntityListData, AppfwOperationError, AppfwRecord } from "../lib/appfwClient";
import {
  gridSelection,
  humanizeFieldName,
  lookupTargetEntity,
  operationBlocker,
  relatedCollectionFields,
  toOperationError
} from "./entityScaffoldModel";
import { recordRouteRef } from "./routeIdentity";

type RelatedCollectionsProps = {
  ownerEntity: AppfwUiEntityContract;
  record: AppfwRecord;
  onNavigateToRecord: (entity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => void;
  onNavigateToNew: (entity: AppfwUiEntityContract, initialValues?: AppfwRecord) => void;
};

export function RelatedCollections({
  ownerEntity,
  record,
  onNavigateToRecord,
  onNavigateToNew
}: RelatedCollectionsProps) {
  const fields = useMemo(() => relatedCollectionFields(ownerEntity), [ownerEntity]);
  if (!fields.length) return null;

  return (
    <div className="crm-related-collections">
      {fields.map((field) => (
        <RelatedCollectionSection
          key={field.name}
          ownerEntity={ownerEntity}
          field={field}
          record={record}
          onNavigateToRecord={onNavigateToRecord}
          onNavigateToNew={onNavigateToNew}
        />
      ))}
    </div>
  );
}

function RelatedCollectionSection({
  ownerEntity,
  field,
  record,
  onNavigateToRecord,
  onNavigateToNew
}: {
  ownerEntity: AppfwUiEntityContract;
  field: AppfwUiFieldContract;
  record: AppfwRecord;
  onNavigateToRecord: (entity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => void;
  onNavigateToNew: (entity: AppfwUiEntityContract, initialValues?: AppfwRecord) => void;
}) {
  const client = useAppfwClient();
  const { auth } = useAuth();
  const { tenant } = useTenant();
  const targetEntity = lookupTargetEntity(field);
  const foreignKey = field.relationship?.fieldName;
  const ownerKey = record[ownerEntity.primaryKey];
  const columns = useMemo(() => (targetEntity ? relatedCollectionColumns(targetEntity, foreignKey) : []), [foreignKey, targetEntity]);
  const selection = useMemo(() => (targetEntity ? gridSelection(targetEntity, columns) : []), [columns, targetEntity]);
  const [data, setData] = useState<AppfwEntityListData | null>(null);
  const [error, setError] = useState<AppfwOperationError | null>(null);
  const createOperation = targetEntity?.operations.find(
    (operation) =>
      operation.kind === "mutation" &&
      operation.returnsShape === "record" &&
      operation.name.startsWith("create_")
  );
  const createBlocker = targetEntity
    ? operationBlocker(createOperation, auth.authorization, tenant.tenantId, "creating")
    : "Related entity is not available.";

  useEffect(() => {
    if (!targetEntity || !foreignKey || ownerKey === null || ownerKey === undefined || ownerKey === "") {
      setData(null);
      setError(null);
      return;
    }

    let active = true;
    setData(null);
    setError(null);
    client
      .queryEntityList(targetEntity, {
        limit: 8,
        selection,
        filter: { [foreignKey]: { _eq: ownerKey } }
      })
      .then((result) => {
        if (active) setData(result.data);
      })
      .catch((caught: unknown) => {
        if (active) setError(toOperationError(caught));
      });

    return () => {
      active = false;
    };
  }, [client, foreignKey, ownerKey, selection, targetEntity]);

  if (!targetEntity || !foreignKey) return null;

  const total = data?.page.queryCount ?? data?.rows.length ?? 0;

  return (
    <section className="crm-related-collection">
      <div className="crm-related-collection-header">
        <div>
          <strong>{field.label}</strong>
          <span>
            {targetEntity.caption.plural} linked to this {ownerEntity.caption.singular.toLowerCase()}
          </span>
        </div>
        <div className="crm-related-collection-actions">
          <Badge>{total} linked</Badge>
          <button
            type="button"
            className="crm-button primary compact"
            disabled={Boolean(createBlocker)}
            title={createBlocker ?? undefined}
            onClick={() => onNavigateToNew(targetEntity, { [foreignKey]: ownerKey })}
          >
            New
          </button>
        </div>
      </div>
      {error ? <div className="crm-field-errors">{error.message}</div> : null}
      {!data && !error ? <div className="crm-relationship-empty">Loading {targetEntity.caption.plural.toLowerCase()}...</div> : null}
      {data && data.rows.length ? (
        <div className="crm-related-collection-grid">
          <DataTable
            columns={columns}
            rows={data.rows}
            rowKey={targetEntity.primaryKey}
            columnLabels={Object.fromEntries(columns.map((column) => [column, humanizeFieldName(column)]))}
            onRowSelect={(row) => onNavigateToRecord(targetEntity, String(row[targetEntity.primaryKey] ?? ""), recordRouteRef(targetEntity, row))}
          />
        </div>
      ) : null}
      {data && !data.rows.length ? (
        <div className="crm-relationship-empty">
          No linked {targetEntity.caption.plural.toLowerCase()} yet.
        </div>
      ) : null}
    </section>
  );
}

function relatedCollectionColumns(entity: AppfwUiEntityContract, foreignKey?: string) {
  const ordered = [entity.captionField, ...entity.scaffold.list.fields]
    .filter((name) => name !== entity.primaryKey && name !== foreignKey)
    .filter((name) => entity.fields.some((field) => field.name === name));
  const columns = Array.from(new Set(ordered)).slice(0, 5);
  return columns.length ? columns : [entity.captionField];
}
