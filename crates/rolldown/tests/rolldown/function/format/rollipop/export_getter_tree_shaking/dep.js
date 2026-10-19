const effects = (globalThis.effects = []);

export const unusedEffect = effects.push('effect');

export const unusedRead = {
  get value() {
    effects.push('read');
    return 42;
  },
}.value;

export const unusedLocal = { value: 'local' };
effects.push(unusedLocal.value);

const removedBinding = 'removed-binding';
export { removedBinding as unusedSpecifier };

export let used = 1;
export function increment() {
  used += 1;
}
