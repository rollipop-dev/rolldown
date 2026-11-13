// Reads the package's exports later, through its index.
import { registered, nested } from './lib/index.js';

export const later = () => [registered, nested];
