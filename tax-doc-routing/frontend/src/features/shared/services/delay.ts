// Artificial latency so mock services behave like real API calls.
export const delay = (ms: number): Promise<void> => new Promise((resolve) => setTimeout(resolve, ms));
