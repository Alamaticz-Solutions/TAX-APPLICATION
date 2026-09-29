// ESLint flat config for the product frontend (`npm run lint`).
// The framework's product-frontend.md lists `npm run lint` in the minimum
// evidence set. General correctness rules come from the standard presets; the
// import restrictions encode the framework's own PDS consumption rules.
import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: ['dist/**', 'target/**', 'node_modules/**', 'vendor/**', 'src/generated/**', '../backend/product_dist/**']
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2022,
      globals: { ...globals.browser, ...globals.es2022 }
    },
    plugins: { 'react-hooks': reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // Framework rule (pds-health-design-system.md): products import only the
      // package's public entry points — never checkout-relative component
      // source, React Aria, or AG Grid — and the product-local `@ui-kit`
      // indirection no longer exists.
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            { group: ['@ui-kit', '@ui-kit/*'], message: 'Import PDS components directly from @appfw/pds-health-components.' },
            { group: ['**/components/src/**', '**/pds_health/components/src/**'], message: 'Products must not import PDS component source; use the package entry points.' },
            { group: ['react-aria', 'react-aria/*', 'react-aria-components', 'react-aria-components/*', '@react-aria/*', '@react-stately/*'], message: 'Products must not import React Aria directly; use the PDS components built on it.' },
            { group: ['ag-grid-community', 'ag-grid-react', 'ag-grid-*'], message: 'Products must not import AG Grid directly; use the PDS DataGrid components.' }
          ]
        }
      ]
    }
  },
  {
    files: ['scripts/**/*.mjs', 'vite.config.ts', 'vitest.config.ts', 'eslint.config.js'],
    languageOptions: { globals: { ...globals.node } }
  }
);
