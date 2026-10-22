import { useState } from 'react';
import { Dialog } from '@appfw/pds-health-components/layout';
import { Button, SegmentedControl } from '@appfw/pds-health-components/primitives';
import { FieldGroup, FormLayout, InlineAlert } from '@appfw/pds-health-components/surfaces';
import { SingleSelectField } from '../../components/ui';
import type { BulkDoc, BulkDocumentValues } from '../../lib/bulkRequests';
import { useDocumentTypeCatalogue } from '../shared/hooks/useDocumentTypeCatalogue';
import { useEntityDirectory } from '../shared/hooks/useEntityDirectory';
import type { EntityType } from '../shared/types';

/** Sets the details of one file in the bulk preview: type, year, entity and password protection. */
export function BulkDocumentDialog({
  doc,
  canProtect,
  onSave,
  onClose
}: {
  doc: BulkDoc;
  canProtect: boolean;
  onSave: (values: BulkDocumentValues) => void;
  onClose: () => void;
}) {
  const { documentTypes, documentYears, entityTypes, getDocumentType } = useDocumentTypeCatalogue();
  const { entitiesOfType, findEntity } = useEntityDirectory();
  const [type, setType] = useState(doc.document_type);
  const [year, setYear] = useState(doc.year ? String(doc.year) : '');
  const [entityType, setEntityType] = useState(doc.entity_type);
  const [entityName, setEntityName] = useState(doc.entity_name);
  const [protect, setProtect] = useState(doc.password_protected);
  const [error, setError] = useState('');
  const cfg = type ? getDocumentType(type) : undefined;
  const supports = Boolean(cfg?.supportsPasswordProtection);
  const entity = findEntity(entityName);

  const save = () => {
    if (!type || !year) {
      setError('Choose a document type and a year.');
      return;
    }
    const belongs = Boolean(cfg?.allowsEntity && entityType && entityName);
    onSave({
      documentType: type,
      year: Number(year),
      entityType: belongs ? entityType : '',
      entityName: belongs ? entityName : '',
      entityNumber: belongs ? entity?.number ?? doc.entity_number : '',
      passwordProtected: protect && supports && canProtect
    });
  };

  return (
    <Dialog
      open
      size="md"
      title="Review Document"
      onClose={onClose}
      footer={
        <>
          <Button variant="quiet" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={save}>
            Save Changes
          </Button>
        </>
      }
    >
      <p className="tax-muted">{doc.file_name}</p>
      {doc.errors.length > 0 ? <InlineAlert tone="warning" title="This file needs details" detail={doc.errors.join(' ')} /> : null}
      <FormLayout columns="one">
        <SingleSelectField
          label="Document Type"
          required
          value={type}
          options={documentTypes.map((d) => ({ value: d.type, label: d.type }))}
          placeholder="Select…"
          onValueChange={setType}
        />
        <SingleSelectField label="Year" required value={year} options={documentYears.map((y) => ({ value: String(y), label: String(y) }))} placeholder="Select…" onValueChange={setYear} />
        {cfg?.allowsEntity ? (
          <>
            <SingleSelectField
              label="Entity Type"
              value={entityType}
              options={entityTypes.map((t) => ({ value: t, label: t }))}
              placeholder="Select…"
              onValueChange={(v) => {
                setEntityType(v);
                setEntityName('');
              }}
            />
            <SingleSelectField
              label="Entity Name"
              value={entityName}
              disabled={!entityType}
              options={entitiesOfType(entityType as EntityType | '').map((e) => ({ value: e.name, label: e.name }))}
              placeholder={entityType ? 'Select…' : 'Choose an entity type first'}
              hint={entity ? `Entity Number ${entity.number}` : undefined}
              onValueChange={setEntityName}
            />
          </>
        ) : null}
        <FieldGroup legend="Password protect the client copies?">
          <SegmentedControl<'yes' | 'no'>
            ariaLabel="Password protect"
            disabled={!supports || !canProtect}
            value={protect && supports && canProtect ? 'yes' : 'no'}
            onValueChange={(v) => setProtect(v === 'yes')}
            options={[
              { value: 'no', label: 'No' },
              { value: 'yes', label: 'Yes' }
            ]}
          />
        </FieldGroup>
      </FormLayout>
      {error ? <InlineAlert tone="danger" title="Check the details" detail={error} /> : null}
    </Dialog>
  );
}
