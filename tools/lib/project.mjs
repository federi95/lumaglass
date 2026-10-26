// Repository paths stay independent of the location of individual host tools.
import path from 'node:path';

export const root = path.resolve(import.meta.dirname, '../..');
