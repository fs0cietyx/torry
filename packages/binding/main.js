import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

let binding;
try {
  // Try platform-specific bindings or fallback
  binding = require('./binding.darwin-arm64.node');
} catch (e) {
  throw new Error(`Failed to load native binding: ${e.message}`);
}

export const RuntimeContext = binding.RuntimeContext;
