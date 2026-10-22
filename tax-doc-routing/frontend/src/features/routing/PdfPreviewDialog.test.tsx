import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PdfPreviewDialog } from './PdfPreviewDialog';

describe('PdfPreviewDialog', () => {
  beforeEach(() => {
    URL.createObjectURL = vi.fn(() => 'blob:preview');
    URL.revokeObjectURL = vi.fn();
  });
  afterEach(() => vi.restoreAllMocks());

  it('shows the file name and the PDF itself, and releases the file when closed', () => {
    const file = new File(['%PDF-1.4'], 'Bose K-1.pdf', { type: 'application/pdf' });
    const { unmount } = render(<PdfPreviewDialog file={file} onClose={() => undefined} />);
    expect(screen.getByText('Bose K-1.pdf')).toBeTruthy();
    expect(screen.getByTitle('Preview of Bose K-1.pdf').getAttribute('src')).toBe('blob:preview');
    unmount();
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:preview');
  });
});
