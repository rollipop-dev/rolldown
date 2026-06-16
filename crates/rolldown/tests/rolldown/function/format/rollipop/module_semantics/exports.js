export default 'default-value';

export const namedValue = 'named-value';

export let mutableValue = 0;

export function incrementMutableValue() {
  mutableValue += 1;
}

export function getMutableValue() {
  return mutableValue;
}
