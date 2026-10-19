export const unusedEffect = (globalThis.effects = ['dep']);
export const unusedPure = 42;
export let value = 1;
export function increment() {
  value += 1;
}
