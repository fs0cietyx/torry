import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  clean: true,
  sourcemap: true,
  target: 'node22',
  // CLI doesn't need .d.ts — it's an executable, not a library
  dts: false,
  banner: {
    // Shebang for the CLI entry point.
    // tsup strips shebangs from source files, so we re-inject it here.
    // This makes `./dist/index.js` directly executable after `chmod +x`.
    js: '#!/usr/bin/env node',
  },
  // Don't bundle workspace deps or native addon — resolve from node_modules at runtime
  external: ['@torry/core', '@torry/shared', '@torry/binding'],
});
