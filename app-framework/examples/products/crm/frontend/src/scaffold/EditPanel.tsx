import { useEffect, useMemo, useState } from "react";
import type { AppfwUiEntityContract } from "../generated/appfw-ui-contract";
import { useAppfwClient, useAuth, useTenant } from "../app/providers";
import {
  Badge,
  Button,
  ConfirmDialog,
  FormLayout,
  FormLoadingPreview,
  StateView,
  Surface,
  ValidationSummary
} from "../components/ui";
import type { AppfwOperationError, AppfwRecord } from "../lib/appfwClient";
import {
  DELETE_CONFIRMATION_DISABLED_REASON,
  deleteInput,
  draftsEqual,
  editDraft,
  formFields,
  formSelectionFields,
  mutationInput,
  newDraft,
  operationBlocker,
  toOperationError,
  validateDraft
} from "./entityScaffoldModel";
import { RecordFormField } from "./RecordFormFields";
import { RelatedCollections } from "./RelatedCollections";
import { useLoadingPreview } from "./useLoadingPreview";

type EditPanelProps = {
  entity: AppfwUiEntityContract;
  recordId: string | null;
  isNew: boolean;
  initialValues?: AppfwRecord;
  onCancel: () => void;
  onDeleted: (record: AppfwRecord) => void;
  onNavigateToRecord: (entity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => void;
  onNavigateToNew: (entity: AppfwUiEntityContract, initialValues?: AppfwRecord) => void;
  onLoaded?: (record: AppfwRecord) => void;
  onSaved: (record: AppfwRecord | null) => void;
};

export function EditPanel({
  entity,
  recordId,
  isNew,
  initialValues,
  onCancel,
  onDeleted,
  onNavigateToRecord,
  onNavigateToNew,
  onLoaded,
  onSaved
}: EditPanelProps) {
  const client = useAppfwClient();
  const { auth } = useAuth();
  const { tenant } = useTenant();
  const [row, setRow] = useState<AppfwRecord | null>(null);
  const [draft, setDraft] = useState<AppfwRecord>({});
  const [initialDraft, setInitialDraft] = useState<AppfwRecord>({});
  const [loadingRecord, setLoadingRecord] = useState(false);
  const [recordLoadError, setRecordLoadError] = useState<AppfwOperationError | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [showClientValidation, setShowClientValidation] = useState(false);
  const [validation, setValidation] = useState<Record<string, string[]> | undefined>();
  const [error, setError] = useState<AppfwOperationError | null>(null);

  const createOperation = entity.operations.find(
    (operation) =>
      operation.kind === "mutation" &&
      operation.returnsShape === "record" &&
      operation.name.startsWith("create_")
  );
  const updateOperation = entity.operations.find(
    (operation) =>
      operation.kind === "mutation" &&
      operation.returnsShape === "record" &&
      operation.name.startsWith("update_")
  );
  const deleteOperation = entity.operations.find(
    (operation) =>
      operation.kind === "mutation" &&
      operation.returnsShape === "scalar" &&
      operation.name.startsWith("delete_")
  );
  const activeSaveOperation = isNew ? createOperation : updateOperation;
  const fields = useMemo(() => formFields(entity), [entity]);
  const formSelection = useMemo(() => formSelectionFields(entity), [entity]);
  const initialValuesKey = useMemo(() => JSON.stringify(initialValues ?? {}), [initialValues]);
  const isDirty = useMemo(() => !draftsEqual(draft, initialDraft), [draft, initialDraft]);
  const clientValidation = useMemo(() => validateDraft(fields, draft), [draft, fields]);
  const hasClientValidation = Object.keys(clientValidation).length > 0;
  const visibleValidation = showClientValidation && hasClientValidation ? clientValidation : validation;
  const saveBlocker = operationBlocker(activeSaveOperation, auth.authorization, tenant.tenantId, isNew ? "creating" : "saving");
  const deleteNeedsConfirmation = deleteOperation?.disabledReason === DELETE_CONFIRMATION_DISABLED_REASON;
  const deleteBlocker = isNew
    ? "New records cannot be deleted until they are saved."
    : operationBlocker(deleteOperation, auth.authorization, tenant.tenantId, "deleting", {
        allowDeleteConfirmationDisabledReason: true
      });
  const saveActionBlocker = normalizeFormChangeBlocker(saveBlocker);
  const deleteActionBlocker = normalizeFormChangeBlocker(deleteBlocker);
  const formBlockers = Array.from(new Set([saveActionBlocker, deleteActionBlocker].filter((blocker): blocker is string => Boolean(blocker))));
  const deleteRecordName = row ? String(row[entity.captionField] ?? row[entity.primaryKey] ?? entity.caption.singular) : entity.caption.singular;
  const showRecordPreview = useLoadingPreview(loadingRecord);

  const navigateToRecord = (targetEntity: AppfwUiEntityContract, recordKey: string, routeRef?: string) => {
    if (isDirty) {
      window.alert("Save or cancel your changes before opening another record.");
      return;
    }
    onNavigateToRecord(targetEntity, recordKey, routeRef);
  };
  const navigateToNew = (targetEntity: AppfwUiEntityContract, values?: AppfwRecord) => {
    if (isDirty) {
      window.alert("Save or cancel your changes before creating a related record.");
      return;
    }
    onNavigateToNew(targetEntity, values);
  };

  useEffect(() => {
    let active = true;
    setShowClientValidation(false);
    setValidation(undefined);
    setError(null);
    setRecordLoadError(null);
    setDeleting(false);
    setDeleteConfirmOpen(false);

    if (isNew) {
      const nextDraft = { ...newDraft(fields), ...(initialValues ?? {}) };
      setRow(null);
      setDraft(nextDraft);
      setInitialDraft(nextDraft);
      setLoadingRecord(false);
      return () => {
        active = false;
      };
    }

    if (!recordId) {
      setRow(null);
      setDraft({});
      setInitialDraft({});
      setLoadingRecord(false);
      return () => {
        active = false;
      };
    }

    setRow(null);
    setDraft({});
    setInitialDraft({});
    setLoadingRecord(true);
    client
      .findEntityRecord(entity, recordId, formSelection)
      .then((result) => {
        if (!active) return;
        const record = result.data.record;
        setRow(record);
        const nextDraft = record ? editDraft(fields, record) : {};
        setDraft(nextDraft);
        setInitialDraft(nextDraft);
        if (record) onLoaded?.(record);
      })
      .catch((caught: unknown) => {
        if (!active) return;
        setRecordLoadError(toOperationError(caught));
      })
      .finally(() => {
        if (active) setLoadingRecord(false);
      });

    return () => {
      active = false;
    };
  }, [client, entity, fields, formSelection, initialValuesKey, isNew, onLoaded, recordId]);

  useEffect(() => {
    if (!isDirty) return undefined;
    const handler = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [isDirty]);

  if (!row && !isNew && !recordId) {
    return <StateView kind="empty" title={`Select a ${entity.caption.singular.toLowerCase()}`} />;
  }

  if (loadingRecord || showRecordPreview) {
    if (!showRecordPreview) {
      return (
        <Surface
          className="crm-form-card crm-form-card--pending"
          aria-busy="true"
          aria-label={`Loading ${entity.caption.singular.toLowerCase()}`}
        >
          <span aria-hidden="true" />
        </Surface>
      );
    }
    return <RecordFormSkeleton entity={entity} fieldCount={fields.length} />;
  }

  if (recordLoadError) {
    return (
      <StateView
        kind={recordLoadError.category === "policy_denied" || recordLoadError.category === "auth" ? "denied" : "error"}
        title={recordLoadError.category === "policy_denied" ? "Access denied" : "Record load failed"}
        detail={recordLoadError.message}
      />
    );
  }

  if (!row && !isNew) {
    return <StateView kind="empty" title={`${entity.caption.singular} not found`} />;
  }

  if ((!activeSaveOperation || entity.scaffold.edit.disabledReason) && !deleteOperation && !isNew) {
    return (
      <StateView
        kind="denied"
        title={isNew ? "Creation unavailable" : "Editing unavailable"}
        detail={entity.scaffold.edit.disabledReason ?? `No generated ${isNew ? "create" : "update"} operation exists for this entity.`}
      />
    );
  }

  if (saving || deleting) {
    return <StateView kind="loading" title={saving ? "Saving" : "Deleting"} />;
  }

  return (
    <Surface className="crm-form-card">
      <form
        className="crm-record-form flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          setShowClientValidation(true);
          if (hasClientValidation) {
            setValidation(undefined);
            setError(null);
            return;
          }
          if (!isDirty) return;
          if (!activeSaveOperation || saveBlocker) {
            setError({
              message: saveActionBlocker ?? `No enabled generated ${isNew ? "create" : "update"} operation exists for this entity.`,
              category: activeSaveOperation?.requiresAuth && !auth.authorization ? "auth" : "policy_denied"
            });
            return;
          }
          setSaving(true);
          setShowClientValidation(false);
          setValidation(undefined);
          setError(null);
          client
            .saveEntityRecord(
              entity,
              isNew ? "create" : "update",
              mutationInput(entity, activeSaveOperation.optimisticConcurrencyField, row, draft),
              formSelection
            )
            .then((result) => {
              const savedRecord = result.data.record;
              if (savedRecord) {
                setRow(savedRecord);
                const nextDraft = editDraft(fields, savedRecord);
                setDraft(nextDraft);
                setInitialDraft(nextDraft);
              }
              onSaved(savedRecord);
            })
            .catch((caught: unknown) => {
              const details = toOperationError(caught);
              setError(details);
              setValidation(details.validation);
            })
            .finally(() => setSaving(false));
        }}
      >
        <div className="crm-form-status">
          <Badge>{isNew ? "New record" : "Existing record"}</Badge>
          <Badge tone={isDirty ? "accent" : "neutral"}>{isDirty ? "Unsaved changes" : "Clean"}</Badge>
          {showClientValidation && hasClientValidation ? <Badge tone="danger">Validation needed</Badge> : null}
        </div>
        {formBlockers.map((blocker) => (
          <div key={blocker} className="crm-form-blocker">
            {blocker}
          </div>
        ))}
        <ValidationSummary validation={visibleValidation} />
        {error && !visibleValidation ? (
          <StateView
            kind={error.category === "policy_denied" || error.category === "auth" ? "denied" : "error"}
            title={error.category === "policy_denied" ? "Access denied" : "Save failed"}
            detail={error.message}
          />
        ) : null}
        <FormLayout className="crm-form-grid">
          {fields.map((field) => (
            <RecordFormField
              key={field.name}
              ownerEntity={entity}
              field={field}
              value={draft[field.name]}
              errors={showClientValidation ? clientValidation[field.name] : undefined}
              onChange={(value) => {
                setDraft((current) => ({ ...current, [field.name]: value }));
                setValidation(undefined);
                setError(null);
                setDeleteConfirmOpen(false);
              }}
              onNavigateToRecord={navigateToRecord}
            />
          ))}
        </FormLayout>
        {!isNew && row ? (
          <RelatedCollections
            ownerEntity={entity}
            record={row}
            onNavigateToRecord={navigateToRecord}
            onNavigateToNew={navigateToNew}
          />
        ) : null}
        <ConfirmDialog
          open={deleteConfirmOpen}
          title={`Delete ${entity.caption.singular}`}
          description={`Delete ${deleteRecordName}?`}
          tone="danger"
          confirmLabel="Delete record"
          isConfirming={deleting}
          confirmDisabled={Boolean(deleteBlocker) || !row || !deleteOperation}
          onCancel={() => setDeleteConfirmOpen(false)}
          onConfirm={() => {
            if (!deleteOperation || deleteBlocker) return;
            if (!row) return;
            setDeleting(true);
            setDeleteConfirmOpen(false);
            setShowClientValidation(false);
            setValidation(undefined);
            setError(null);
            client
              .deleteEntityRecord(entity, deleteInput(entity, deleteOperation.optimisticConcurrencyField, row), {
                confirmed: true
              })
              .then(() => onDeleted(row))
              .catch((caught: unknown) => {
                const details = toOperationError(caught);
                setError(details);
                setValidation(details.validation);
              })
              .finally(() => setDeleting(false));
          }}
        >
          <strong>Delete {deleteRecordName}?</strong>
          <span>This action cannot be undone.</span>
          {deleteNeedsConfirmation ? <span>Generated delete actions require explicit product confirmation.</span> : null}
        </ConfirmDialog>
        <div className="crm-form-actions">
          <Button
            type="button"
            variant="danger"
            disabled={Boolean(deleteBlocker) || isDirty}
            title={deleteActionBlocker ?? (isDirty ? "Save or discard changes before deleting." : undefined)}
            onClick={() => {
              if (!deleteOperation || deleteBlocker) return;
              if (!row) return;
              setDeleteConfirmOpen(true);
            }}
          >
            Delete
          </Button>
          <Button
            type="button"
            variant="secondary"
            disabled={!isDirty}
            title={!isDirty ? "No unsaved changes to discard." : undefined}
            onClick={() => {
              if (isDirty && !window.confirm("Discard unsaved changes?")) return;
              setDraft(initialDraft);
              setShowClientValidation(false);
              setValidation(undefined);
              setError(null);
              onCancel();
            }}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={!isDirty || Boolean(saveBlocker) || !activeSaveOperation}
            title={saveActionBlocker ?? (!activeSaveOperation ? `No enabled generated ${isNew ? "create" : "update"} operation exists for this entity.` : undefined)}
          >
            {isNew ? "Create record" : "Save changes"}
          </Button>
        </div>
      </form>
    </Surface>
  );
}

function RecordFormSkeleton({
  entity,
  fieldCount
}: {
  entity: AppfwUiEntityContract;
  fieldCount: number;
}) {
  const previewFields = Array.from({ length: Math.min(8, Math.max(4, fieldCount)) });

  return (
    <Surface className="crm-form-card crm-form-card--skeleton">
      <FormLoadingPreview
        fieldCount={previewFields.length}
        label={`Loading ${entity.caption.singular.toLowerCase()}`}
      />
    </Surface>
  );
}

function normalizeFormChangeBlocker(blocker: string | null) {
  if (!blocker) return null;
  if (blocker.startsWith("Sign in before ")) return "Sign in to make changes";
  return blocker;
}
