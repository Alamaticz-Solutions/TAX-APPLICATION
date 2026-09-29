import { useEffect, useMemo, useState } from "react";
import type { AppfwUiFieldContract } from "../generated/appfw-ui-contract";
import { useAppfwClient } from "../app/providers";
import type { LookupState, RelationshipSelectorMode } from "./types";
import {
  lookupOptionsFromRows,
  lookupSelection,
  lookupSort,
  lookupTargetEntity,
  toOperationError
} from "./entityScaffoldModel";

export function useLookupOptions(
  field: AppfwUiFieldContract,
  enabled: boolean,
  mode: RelationshipSelectorMode = "lookup",
  selectedKeys: readonly string[] = []
) {
  const client = useAppfwClient();
  const targetEntity = useMemo(() => lookupTargetEntity(field), [field]);
  const [state, setState] = useState<LookupState>({ status: "idle", options: [], rows: [] });
  const selectedKeySignature = selectedKeys.join("\u001f");

  useEffect(() => {
    if (!enabled || !targetEntity) {
      setState({ status: "idle", options: [], rows: [] });
      return;
    }

    let active = true;
    setState({ status: "loading", options: [], rows: [] });
    const selection = lookupSelection(targetEntity, mode);
    const currentKeys = selectedKeySignature.split("\u001f").filter(Boolean);
    const candidateRequest = client.queryEntityList(targetEntity, {
        limit: 100,
        selection,
        sort: lookupSort(targetEntity)
      });
    const selectedRequest = currentKeys.length
      ? client.queryEntityList(targetEntity, {
          limit: Math.max(currentKeys.length, 1),
          selection,
          filter: selectedRecordFilter(targetEntity.primaryKey, currentKeys)
        })
      : Promise.resolve(null);

    Promise.all([candidateRequest, selectedRequest])
      .then(([candidateResult, selectedResult]) => {
        if (!active) return;
        const rows = mergeLookupRows(targetEntity.primaryKey, [
          ...(selectedResult?.data.rows ?? []),
          ...candidateResult.data.rows
        ]);
        setState({
          status: "loaded",
          options: lookupOptionsFromRows(targetEntity, rows),
          rows
        });
      })
      .catch((caught: unknown) => {
        if (!active) return;
        setState({
          status: "error",
          options: [],
          rows: [],
          error: toOperationError(caught).message || `Could not load ${targetEntity.caption.plural.toLowerCase()}.`
        });
      });

    return () => {
      active = false;
    };
  }, [client, enabled, mode, selectedKeySignature, targetEntity]);

  return { ...state, targetEntity };
}

function selectedRecordFilter(primaryKey: string, selectedKeys: readonly string[]) {
  if (selectedKeys.length === 1) return { [primaryKey]: { _eq: selectedKeys[0] } };
  return { [primaryKey]: { _in: selectedKeys } };
}

function mergeLookupRows(primaryKey: string, rows: LookupState["rows"]) {
  const seen = new Set<string>();
  return rows.filter((row) => {
    const key = String(row[primaryKey] ?? "");
    if (!key || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
