import { namedValue as importedNamed } from './exports.js';

const localValue = 'local-value';
const source = { value: 'destructured-value' };

export { importedNamed as importedThenExported };
export { localValue as localAlias };

export const { value: destructuredValue } = source;

export { destructuredValue as 'dashed-name' };

export function fnExport() {
  return 'fn-export';
}

export class ClassExport {
  value() {
    return 'class-export';
  }
}
