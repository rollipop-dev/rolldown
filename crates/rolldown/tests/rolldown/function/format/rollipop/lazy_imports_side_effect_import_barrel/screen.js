// Reads the package's exports later, through its index.
import { registered, nested } from './pkg/index.js';

export const later = () => [registered, nested];
