export const pdsEnterpriseCopy = {
  loading: "Loading...",
  empty: "No records found",
  deniedTitle: "Access is restricted",
  deniedDetail: "Your current role or tenant context does not allow this action.",
  validationTitle: "Review the highlighted fields",
  unexpectedTitle: "Something went wrong",
  retry: "Try again",
  confirmDestructive: "This action cannot be undone."
} as const;

export type PdsEnterpriseCopyKey = keyof typeof pdsEnterpriseCopy;
