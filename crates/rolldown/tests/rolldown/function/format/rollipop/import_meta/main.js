export {};
globalThis.result = [typeof import.meta, import.meta.url, import.meta.unknown, import.meta.hot];
if (import.meta.hot) throw new Error('HMR must be disabled in production');
