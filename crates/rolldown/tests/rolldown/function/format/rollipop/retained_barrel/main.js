import { value, count, increment, local, namedDefault } from './kept.js';
increment();
globalThis.result = [value.answer, count, local, namedDefault];
