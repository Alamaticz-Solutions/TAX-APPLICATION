// Processing timeline (business spec sections 5-8). Wording lives here so the storage
// platform name can change (Box -> SharePoint) in one place.
export const STORAGE_NAME = 'Box';
// The folder under each year in the client's Internal folder that holds the copies released to the
// client. Pega hardcodes this name; the backend reads it from RELEASED_TO_CLIENT_FOLDER_NAME (same
// default), so keep the two in step.
export const RELEASED_TO_CLIENT_FOLDER = '6. Released to Client';

export type ProcessingStep = { key: string; label: string; active: string };

export const PROCESSING_STEPS: readonly ProcessingStep[] = [
  { key: 'created', label: 'Routing record created', active: 'Creating routing record' },
  { key: 'staging', label: 'Temporary staging folder created', active: 'Creating temporary staging folder' },
  { key: 'uploaded', label: 'Documents uploaded', active: 'Uploading documents' },
  { key: 'verify', label: `Verifying client ${STORAGE_NAME} folder`, active: `Verifying client ${STORAGE_NAME} folder` },
  { key: 'processing', label: 'Processing documents', active: 'Applying password protection where required' },
  { key: 'moving', label: 'Moving documents', active: 'Moving documents to permanent folders' },
  { key: 'cleanup', label: 'Cleaning temporary folder', active: 'Deleting temporary staging folder' },
  { key: 'notify', label: 'Sending confirmation', active: 'Sending client confirmation' },
  { key: 'done', label: 'Completed', active: 'Finalising record' }
];

export const STEP_DURATION_MS = 1100;
export const ESTIMATED_MINUTES = 10;
