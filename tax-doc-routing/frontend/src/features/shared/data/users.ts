import type { AppUser, Role } from '../types';

// The two role-switcher identities (business spec 2.2's two roles). `id` is filled in with the
// real backend AppUser id once TaxRoutingProvider's permissions fetch resolves (see BACKEND_USER_NAME
// there) — everyone else who might be assigned a record comes from useStaffDirectory, read live
// from the database instead of a hardcoded list.
export const users: Record<Role, AppUser> = {
  standard: { id: '', name: 'Alex Morgan', initials: 'AM', role: 'standard', roleLabel: 'Tax Staff' },
  admin: { id: '', name: 'Dana Whitfield', initials: 'DW', role: 'admin', roleLabel: 'Tax Admin' }
};
