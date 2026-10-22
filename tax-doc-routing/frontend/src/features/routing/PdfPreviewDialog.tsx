import { useEffect, useState } from 'react';
import { Dialog } from '@appfw/pds-health-components/layout';
import { Button } from '@appfw/pds-health-components/primitives';
import { ButtonLink } from '@appfw/pds-health-components/work-surfaces';
import { formatBytes } from '../shared/utils/format';

/**
 * Shows a PDF so the user can check it is the right document, and download it. The file comes from
 * this browser (screen 2) or was read from storage by the caller (the case page).
 */
export function PdfPreviewDialog({ file, onClose }: { file: File; onClose: () => void }) {
  const [url, setUrl] = useState('');
  useEffect(() => {
    const objectUrl = URL.createObjectURL(file);
    setUrl(objectUrl);
    return () => URL.revokeObjectURL(objectUrl);
  }, [file]);

  return (
    <Dialog
      open
      size="lg"
      title={file.name}
      description={formatBytes(file.size)}
      onClose={onClose}
      footer={
        <>
          {url ? (
            <ButtonLink variant="secondary" href={url} download={file.name}>
              Download
            </ButtonLink>
          ) : null}
          <Button variant="primary" onClick={onClose}>
            Close
          </Button>
        </>
      }
    >
      {url ? <iframe className="tax-pdf-preview" title={`Preview of ${file.name}`} src={url} /> : null}
    </Dialog>
  );
}
