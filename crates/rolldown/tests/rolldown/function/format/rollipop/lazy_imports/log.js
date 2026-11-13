export function log(entry) {
  (globalThis.__rollipop_lazy_log ??= []).push(entry);
}
