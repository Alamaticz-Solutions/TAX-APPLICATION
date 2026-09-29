import { useState } from 'react';
import { CheckboxField, TextField } from '@appfw/pds-health-components/forms';
import { Dialog } from '@appfw/pds-health-components/layout';
import { Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { FieldGroup, FormLayout, InlineAlert } from '@appfw/pds-health-components/surfaces';
import { FORM_8308_ATTACHMENT_OPTIONS, K1_ATTACHMENT_OPTIONS } from '../shared/config/documentTypes';
import { SingleSelectField } from '../../components/ui';
import { useDocumentTypeCatalogue } from '../shared/hooks/useDocumentTypeCatalogue';
import { useEntityDirectory } from '../shared/hooks/useEntityDirectory';
import type { DocumentTypeName, EntityType, TaxDocument } from '../shared/types';
import { attachmentGroups } from '../shared/utils/routingRules';
import { validateDocument, type DocumentFormErrors } from '../shared/utils/validation';

type FormState = {
  type: DocumentTypeName | '';
  year: string;
  name: string;
  belongsToEntity: 'yes' | 'no';
  entityType: EntityType | '';
  entityName: string;
  passwordProtected: boolean;
  attachments: Record<string, boolean>;
};

export type EditedDocument = Pick<TaxDocument, 'type' | 'year' | 'name' | 'entity' | 'passwordProtected' | 'attachments'>;

function toForm(doc: TaxDocument | null, fileName: string): FormState {
  return {
    type: doc?.type ?? '',
    year: doc ? String(doc.year) : '',
    name: doc?.name ?? fileName,
    belongsToEntity: doc?.entity ? 'yes' : 'no',
    entityType: doc?.entity?.type ?? '',
    entityName: doc?.entity?.name ?? '',
    passwordProtected: doc?.passwordProtected ?? false,
    attachments: doc?.attachments ?? {}
  };
}

/**
 * Review and correct one uploaded document. Used for AI-prefilled rows and for files the
 * assistant could not classify (`doc` is null and every field starts blank).
 */
export function EditDocumentDialog({
  doc,
  fileName,
  unclassified,
  onSave,
  onClose
}: {
  doc: TaxDocument | null;
  fileName: string;
  unclassified?: boolean;
  onSave: (values: EditedDocument) => void;
  onClose: () => void;
}) {
  const { getDocumentType, documentTypes, entityTypes, documentYears } = useDocumentTypeCatalogue();
  const { entitiesOfType, findEntity } = useEntityDirectory();
  const typeOptions = documentTypes.map((d) => ({ value: d.type, label: d.type }));
  const yearOptions = documentYears.map((y) => ({ value: String(y), label: String(y) }));
  const entityTypeOptions = entityTypes.map((t) => ({ value: t, label: t }));

  const [form, setForm] = useState<FormState>(() => toForm(doc, fileName));
  const [errors, setErrors] = useState<DocumentFormErrors>({});
  const set = (patch: Partial<FormState>) => setForm((f) => ({ ...f, ...patch }));
  const cfg = form.type ? getDocumentType(form.type) : undefined;
  const entity = findEntity(form.entityName);
  const groups = attachmentGroups(form.type, form.belongsToEntity === 'yes' ? form.entityType : '');

  const save = () => {
    // The file itself was already accepted at upload; only the metadata is validated here.
    const errs = validateDocument({ ...form, file: null });
    delete errs.file;
    setErrors(errs);
    if (Object.keys(errs).length) return;
    onSave({
      type: form.type as DocumentTypeName,
      year: Number(form.year),
      name: form.name.trim(),
      entity: cfg?.allowsEntity && form.belongsToEntity === 'yes' && form.entityType ? { type: form.entityType, name: form.entityName, number: entity?.number ?? '' } : null,
      passwordProtected: cfg?.supportsPasswordProtection ? form.passwordProtected : false,
      attachments: form.attachments
    });
  };

  return (
    <Dialog
      open
      size="md"
      title={unclassified ? 'Classify Document' : 'Review Document'}
      onClose={onClose}
      footer={
        <>
          <Button variant="quiet" onClick={onClose}>
            {unclassified ? 'Skip file' : 'Cancel'}
          </Button>
          <Button variant="primary" onClick={save}>
            {unclassified ? 'Add Document' : 'Save Changes'}
          </Button>
        </>
      }
    >
      {unclassified ? (
        <InlineAlert tone="warning" title="AI could not classify this file" detail={`Choose the details for ${fileName} yourself.`} />
      ) : (
        <p className="tax-muted">{fileName}</p>
      )}
      <FormLayout columns="one">
        <SingleSelectField
          label="Document Type"
          required
          value={form.type}
          error={errors.type}
          options={typeOptions}
          placeholder="Select…"
          hint={cfg ? `${cfg.businessName} → ${cfg.externalOnly ? `${cfg.externalFolder} (external only)` : `${cfg.externalFolder} + ${cfg.internalFolder}`}` : undefined}
          onValueChange={(v) => set({ type: v as DocumentTypeName | '', attachments: {} })}
        />
        <SingleSelectField label="Year" required value={form.year} error={errors.year} options={yearOptions} placeholder="Select…" onValueChange={(v) => set({ year: v })} />
        <TextField label="Document Name" required value={form.name} error={errors.name} hint="e.g. K1_2024.pdf" onChange={(e) => set({ name: e.target.value })} />

        {cfg?.allowsEntity ? (
          <FieldGroup legend="Does this belong to an Entity?">
            <SegmentedControl<'yes' | 'no'>
              ariaLabel="Belongs to an entity"
              value={form.belongsToEntity}
              onValueChange={(v) => set({ belongsToEntity: v, ...(v === 'no' ? { entityType: '' as const, entityName: '' } : {}) })}
              options={[
                { value: 'no', label: 'No' },
                { value: 'yes', label: 'Yes' }
              ]}
            />
          </FieldGroup>
        ) : null}
        {cfg?.allowsEntity && form.belongsToEntity === 'yes' ? (
          <>
            <SingleSelectField
              label="Entity Type"
              required
              value={form.entityType}
              error={errors.entityType}
              options={entityTypeOptions}
              placeholder="Select…"
              onValueChange={(v) => set({ entityType: v as EntityType | '', entityName: '' })}
            />
            <SingleSelectField
              label="Entity Name"
              required
              value={form.entityName}
              error={errors.entityName}
              disabled={!form.entityType}
              options={entitiesOfType(form.entityType).map((e) => ({ value: e.name, label: e.name }))}
              placeholder={form.entityType ? 'Select entity…' : 'Select an entity type first'}
              onValueChange={(v) => set({ entityName: v })}
            />
            <TextField label="Entity Number" value={entity?.number ?? ''} readOnly hint="Filled in from the selected entity." />
          </>
        ) : null}
        {cfg?.supportsPasswordProtection ? (
          <CheckboxField
            label="Password Protect Client Copies"
            detail="The PDF is encrypted with the client's password."
            checked={form.passwordProtected}
            onChange={(e) => set({ passwordProtected: e.target.checked })}
          />
        ) : null}
        {groups.map((g) => (
          <FieldGroup key={g} legend={g === 'k1' ? 'K-1 Attachments' : 'Form 8308 Attachments'}>
            {(g === 'k1' ? K1_ATTACHMENT_OPTIONS : FORM_8308_ATTACHMENT_OPTIONS).map((o) => (
              <CheckboxField
                key={o.key}
                label={o.label}
                checked={Boolean(form.attachments[o.key])}
                onChange={(e) => set({ attachments: { ...form.attachments, [o.key]: e.target.checked } })}
              />
            ))}
          </FieldGroup>
        ))}
      </FormLayout>
    </Dialog>
  );
}
