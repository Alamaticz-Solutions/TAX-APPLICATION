import { beforeEach, describe, expect, it } from 'vitest';
import { normalizeUserName, readSessionUser, SESSION_STORAGE_KEY, writeSessionUser } from './session';

describe('session user', () => {
  beforeEach(() => window.sessionStorage.clear());

  it('is signed out until a user is written', () => {
    expect(readSessionUser()).toBeNull();
  });

  it('round-trips the user name, trimmed and lower-cased', () => {
    writeSessionUser('  Alex ');
    expect(readSessionUser()).toBe('alex');
    expect(window.sessionStorage.getItem(SESSION_STORAGE_KEY)).toBe('alex');
  });

  it('clears the session on sign out', () => {
    writeSessionUser('dana');
    writeSessionUser(null);
    expect(readSessionUser()).toBeNull();
  });

  it('treats blank input as no user', () => {
    expect(normalizeUserName('   ')).toBeNull();
    expect(normalizeUserName(undefined)).toBeNull();
    writeSessionUser('   ');
    expect(readSessionUser()).toBeNull();
  });
});
