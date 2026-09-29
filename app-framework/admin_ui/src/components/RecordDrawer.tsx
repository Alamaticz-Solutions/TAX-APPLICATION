import {
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  Clock3,
  FileCode2,
  History,
  Info,
  Loader2,
  Play,
  Save,
  ShieldCheck,
  Trash2,
  UserRound
} from "lucide-react";
import { Button, Drawer } from "@appfw/pds-health-components";
import type {
  AuditTimelineEvent,
  AuditTimelineStatus,
  CustomMethodPreview,
  CustomMethodStatus,
  DrawerMode,
  EntityType,
  FormQueryPreview,
  FormActionStatus,
  LookupOption,
  PropertyType,
  RecordDiff,
  RecordValue,
  RelationshipAction,
  SaveMutationPreview
} from "../types";
import { formProps, formatGraphqlVariables, normalizeInputValue } from "../lib/form";
import { hasMethod, lookupKeyForProperty } from "../lib/entityModel";
import { formatResponseTime } from "../lib/timing";
import { FieldInput } from "./FieldInput";

type RecordDrawerProps = {
  entity: EntityType;
  mode: DrawerMode;
  isReadOnly?: boolean;
  formValues: RecordValue;
  actionStatus: FormActionStatus | null;
  auditTimeline: AuditTimelineStatus | null;
  diffs: RecordDiff[];
  isDirty: boolean;
  queryPreview: FormQueryPreview | null;
  lookupOptions: Record<string, LookupOption[]>;
  loadingLookups: Record<string, boolean>;
  mutationPreview: SaveMutationPreview | null;
  customMethodPreviews: CustomMethodPreview[];
  customMethodStatuses: Record<string, CustomMethodStatus>;
  relationshipActions: RelationshipAction[];
  onClose: () => void;
  onDelete: () => void;
  onSave: () => void;
  onExecuteCustomMethod: (preview: CustomMethodPreview) => void;
  onUpdateField: (prop: PropertyType, value: unknown) => void;
  onNavigateRelationship: (action: RelationshipAction) => void;
};

export function RecordDrawer({
  entity,
  mode,
  isReadOnly = false,
  formValues,
  actionStatus,
  auditTimeline,
  diffs,
  isDirty,
  queryPreview,
  lookupOptions,
  loadingLookups,
  mutationPreview,
  customMethodPreviews,
  customMethodStatuses,
  relationshipActions,
  onClose,
  onDelete,
  onSave,
  onExecuteCustomMethod,
  onUpdateField,
  onNavigateRelationship
}: RecordDrawerProps) {
  const canSave = !isReadOnly && hasMethod(entity, mode === "create" ? "Create" : "Update");
  const canDelete = !isReadOnly && mode !== "create" && hasMethod(entity, "Delete");
  const isBusy = Boolean(actionStatus?.isBusy);
  const saveTitle = isReadOnly
    ? "Audit records are read-only"
    : !canSave
    ? `${mode === "create" ? "Create" : "Update"} is not enabled for this entity`
    : isDirty
      ? "Save"
      : mode === "create"
        ? "Enter values before saving"
        : "No changes to save";
  const cancelTitle = isDirty ? "Discard changes" : "No changes to cancel";

  return (
    <Drawer
      open
      className={`record-drawer ${actionStatus ? "has-status" : ""}`}
      size="lg"
      title={entity.caption_1}
      description={isReadOnly ? "View" : mode === "create" ? "Create" : "Edit"}
      onClose={onClose}
      footer={(
        <>
          {isReadOnly ? (
            <span />
          ) : (
            <Button variant="danger" disabled={!canDelete || isBusy} onClick={onDelete}>
              {actionStatus?.isBusy && actionStatus.action === "delete" ? <Loader2 className="spin" size={16} /> : <Trash2 size={16} />}
              Delete
            </Button>
          )}
          <div>
            {isReadOnly ? (
              <Button disabled={isBusy} onClick={onClose}>
                Close
              </Button>
            ) : (
              <>
                <Button disabled={!isDirty || isBusy} onClick={onClose} title={cancelTitle}>
                  Cancel
                </Button>
                <Button variant="primary" disabled={!canSave || !isDirty || isBusy} onClick={onSave} title={saveTitle}>
                  {actionStatus?.isBusy && actionStatus.action === "save" ? <Loader2 className="spin" size={16} /> : <Save size={16} />}
                  Save
                </Button>
              </>
            )}
          </div>
        </>
      )}
    >
        {actionStatus && (
          <div className={`drawer-status ${actionStatus.tone}`}>
            {actionStatus.isBusy ? (
              <Loader2 className="spin" size={16} />
            ) : actionStatus.tone === "error" ? (
              <AlertCircle size={16} />
            ) : actionStatus.tone === "idle" ? (
              <Info size={16} />
            ) : (
              <CheckCircle2 size={16} />
            )}
            <span>{actionStatus.message}</span>
            {actionStatus.responseMs !== null && actionStatus.responseMs !== undefined && (
              <strong>Response time: {formatResponseTime(actionStatus.responseMs)}</strong>
            )}
          </div>
        )}

        {isReadOnly && (
          <div className="drawer-status idle">
            <Info size={16} />
            <span>Audit records are read-only history and cannot be edited or deleted.</span>
          </div>
        )}

        <div className="form-grid">
          {formProps(entity, mode === "create", isReadOnly).map((prop) => (
            <FieldInput
              key={prop.id}
              prop={prop}
              value={formValues[prop.name]}
              mode={mode}
              options={lookupOptions[lookupKeyForProperty(prop)]}
              isLoadingOptions={Boolean(loadingLookups[lookupKeyForProperty(prop)])}
              isDisabled={isBusy}
              isReadOnly={isReadOnly}
              onChange={(value) => onUpdateField(prop, value)}
            />
          ))}
          {diffs.length > 0 && (
            <section className="diff-panel span-2">
              <div>
                <strong>Pending Changes</strong>
                <span>{diffs.length} changed field{diffs.length === 1 ? "" : "s"}</span>
              </div>
              <div className="diff-list">
                {diffs.map((diff) => (
                  <div className="diff-row" key={diff.prop.id}>
                    <b>{diff.prop.caption || diff.prop.name}</b>
                    <span title={formatDiffValue(diff.before, diff.prop)}>{formatDiffValue(diff.before, diff.prop) || "empty"}</span>
                    <em>-&gt;</em>
                    <span title={formatDiffValue(diff.after, diff.prop)}>{formatDiffValue(diff.after, diff.prop) || "empty"}</span>
                  </div>
                ))}
              </div>
            </section>
          )}
          {queryPreview && (
            <details className="mutation-preview span-2">
              <summary>
                <span>
                  <FileCode2 size={15} />
                  GraphQL Query
                </span>
                <em>{queryPreview.operation}</em>
              </summary>
              <div className="mutation-block">
                <strong>Document</strong>
                <pre>{queryPreview.query}</pre>
              </div>
              <div className="mutation-block">
                <strong>Variables</strong>
                <pre>{formatGraphqlVariables(queryPreview.variables)}</pre>
              </div>
            </details>
          )}
          {!isReadOnly && mutationPreview && (
            <details className="mutation-preview span-2">
              <summary>
                <span>
                  <FileCode2 size={15} />
                  GraphQL Mutation
                </span>
                <em>{mutationPreview.operation}</em>
              </summary>
              <div className="mutation-block">
                <strong>Document</strong>
                <pre>{mutationPreview.query}</pre>
              </div>
              <div className="mutation-block">
                <strong>Variables</strong>
                <pre>{formatGraphqlVariables(mutationPreview.variables)}</pre>
              </div>
            </details>
          )}
          {customMethodPreviews.length > 0 && (
            <details className="mutation-preview span-2">
              <summary>
                <span>
                  <FileCode2 size={15} />
                  GraphQL Custom Methods
                </span>
                <em>{customMethodPreviews.length} method{customMethodPreviews.length === 1 ? "" : "s"}</em>
              </summary>
              <div className="custom-method-list">
                {customMethodPreviews.map((preview) => (
                  <CustomMethodCard
                    key={preview.method.name}
                    preview={preview}
                    status={customMethodStatuses[preview.method.name]}
                    onExecute={() => onExecuteCustomMethod(preview)}
                  />
                ))}
              </div>
            </details>
          )}
          {relationshipActions.length > 0 && (
            <section className="relationship-panel span-2">
              <div>
                <strong>Relationships</strong>
                <span>{relationshipActions.length} navigation paths</span>
              </div>
              <div className="relationship-list">
                {relationshipActions.map((action) => (
                  <button
                    className="relationship-row"
                    disabled={action.disabled}
                    key={action.prop.id}
                    onClick={() => onNavigateRelationship(action)}
                    title={action.reason ?? `Open ${action.target?.caption_n ?? action.title}`}
                    type="button"
                  >
                    <span>
                      <b>{action.title}</b>
                      <small>{action.subtitle}</small>
                    </span>
                    <ChevronRight size={17} />
                  </button>
                ))}
              </div>
            </section>
          )}
          {mode === "edit" && auditTimeline && (
            <section className="audit-panel span-2">
              <div className="audit-head">
                <span>
                  <History size={16} />
                  <strong>Audit Timeline</strong>
                </span>
                {auditTimeline.currentPolicy && (
                  <em className={auditTimeline.currentPolicy.allow ? "ok" : "error"}>
                    Current policy: {auditTimeline.currentPolicy.allow ? "allowed" : "denied"}
                  </em>
                )}
              </div>
              <div className={`audit-status ${auditTimeline.tone}`}>
                {auditTimeline.isLoading ? (
                  <Loader2 className="spin" size={15} />
                ) : auditTimeline.tone === "error" ? (
                  <AlertCircle size={15} />
                ) : auditTimeline.tone === "ok" ? (
                  <CheckCircle2 size={15} />
                ) : (
                  <Info size={15} />
                )}
                <span>{auditTimeline.message}</span>
                {auditTimeline.responseMs !== null && auditTimeline.responseMs !== undefined && (
                  <strong>Response time: {formatResponseTime(auditTimeline.responseMs)}</strong>
                )}
              </div>

              {auditTimeline.events.length > 0 ? (
                <div className="audit-events">
                  {auditTimeline.events.map((event, index) => (
                    <AuditEventCard entity={entity} event={event} key={event.audit_id ?? `${event.occurred_at}-${index}`} />
                  ))}
                </div>
              ) : (
                !auditTimeline.isLoading && (
                  <div className="empty-audit">
                    <History size={18} />
                    <span>No audit events found for this record.</span>
                  </div>
                )
              )}
            </section>
          )}
        </div>
    </Drawer>
  );
}

function CustomMethodCard({
  preview,
  status,
  onExecute
}: {
  preview: CustomMethodPreview;
  status?: CustomMethodStatus;
  onExecute: () => void;
}) {
  return (
    <article className="custom-method-card">
      <div className="custom-method-head">
        <span>
          <strong>{preview.method.name}</strong>
          <small>{preview.method.kind} returns {preview.method.return_type}</small>
        </span>
        <Button disabled={status?.isBusy} onClick={onExecute}>
          {status?.isBusy ? <Loader2 className="spin" size={15} /> : <Play size={15} />}
          Execute
        </Button>
      </div>
      {preview.method.args.length > 0 && (
        <div className="custom-method-args">
          {preview.method.args.map((arg) => (
            <span key={arg.name}>
              <b>{arg.name}</b>
              <em>{arg.arg_type}</em>
            </span>
          ))}
        </div>
      )}
      {status && (
        <div className={`custom-method-status ${status.tone}`}>
          {status.isBusy ? (
            <Loader2 className="spin" size={14} />
          ) : status.tone === "error" ? (
            <AlertCircle size={14} />
          ) : status.tone === "ok" ? (
            <CheckCircle2 size={14} />
          ) : (
            <Info size={14} />
          )}
          <span>{status.message}</span>
          {status.responseMs !== null && status.responseMs !== undefined && (
            <strong>Response time: {formatResponseTime(status.responseMs)}</strong>
          )}
        </div>
      )}
      <div className="mutation-block">
        <strong>Document</strong>
        <pre>{preview.query}</pre>
      </div>
      <div className="mutation-block">
        <strong>Variables</strong>
        <pre>{formatGraphqlVariables(preview.variables)}</pre>
      </div>
      {status?.data !== undefined && (
        <div className="mutation-block">
          <strong>Response</strong>
          <pre>{JSON.stringify(status.data, null, 2)}</pre>
        </div>
      )}
    </article>
  );
}

function formatDiffValue(value: unknown, prop: PropertyType) {
  const normalized = normalizeInputValue(value, prop.data_type);
  return normalized.length > 90 ? `${normalized.slice(0, 87)}...` : normalized;
}

function AuditEventCard({ entity, event }: { entity: EntityType; event: AuditTimelineEvent }) {
  const fields = auditFields(entity, event);
  return (
    <article className="audit-event">
      <div className="audit-event-head">
        <span className={`audit-action ${String(event.action ?? "").toLowerCase()}`}>
          {formatAuditAction(event.action)}
        </span>
        <strong>
          <Clock3 size={14} />
          {formatAuditTime(event.occurred_at)}
        </strong>
        <span>
          <UserRound size={14} />
          {formatAuditActor(event)}
        </span>
      </div>

      {fields.length > 0 ? (
        <div className="audit-field-list">
          {fields.map((field) => (
            <div className="audit-field-row" key={field.name}>
              <b>{field.label}</b>
              <span title={field.before}>{field.before || "empty"}</span>
              <em>to</em>
              <span title={field.after}>{field.after || "empty"}</span>
            </div>
          ))}
        </div>
      ) : (
        <div className="audit-empty-fields">No field-level changes were captured.</div>
      )}

      <details className="audit-policy-block">
        <summary>
          <ShieldCheck size={14} />
          Policy context
        </summary>
        <pre>{formatPolicyContext(event)}</pre>
      </details>
    </article>
  );
}

function auditFields(entity: EntityType, event: AuditTimelineEvent) {
  if (!event.diff_json || typeof event.diff_json !== "object") return [];
  return Object.entries(event.diff_json).map(([name, value]) => ({
    name,
    label: fieldLabel(entity, name),
    before: formatAuditValue(value?.before),
    after: formatAuditValue(value?.after)
  }));
}

function fieldLabel(entity: EntityType, name: string) {
  const prop = entity.props.find((item) => item.name === name);
  return prop?.caption || prop?.name || name;
}

function formatAuditValue(value: unknown) {
  if (value === null || value === undefined || value === "") return "";
  if (typeof value === "string") return value.length > 120 ? `${value.slice(0, 117)}...` : value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  try {
    const rendered = JSON.stringify(value);
    return rendered.length > 120 ? `${rendered.slice(0, 117)}...` : rendered;
  } catch {
    return String(value);
  }
}

function formatAuditAction(action: unknown) {
  const text = String(action || "event").replace(/[_-]+/g, " ");
  return text.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatAuditTime(value: unknown) {
  if (typeof value !== "string" || !value) return "Unknown time";
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return value;
  return new Date(timestamp).toLocaleString();
}

function formatAuditActor(event: AuditTimelineEvent) {
  const actor = event.actor_user_name || "unknown actor";
  const roles = Array.isArray(event.actor_roles) && event.actor_roles.length ? ` (${event.actor_roles.join(", ")})` : "";
  return `${actor}${roles}`;
}

function formatPolicyContext(event: AuditTimelineEvent) {
  const payload = {
    outcome: event.outcome ?? null,
    policy: event.policy_json ?? null,
    redactions: event.redactions_json ?? null,
    event_hash: event.event_hash ?? null,
    prev_hash: event.prev_hash ?? null
  };
  return JSON.stringify(payload, null, 2);
}
