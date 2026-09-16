import { group } from './outer.js';
import { group as keptGroup, local } from './kept.js';
globalThis.result = [group.value(), keptGroup.value(), local];
