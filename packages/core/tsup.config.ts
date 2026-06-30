import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['esm'],
  dts: true,
  clean: true,
  sourcemap: true,
  target: 'node22',
  // Don't bundle the binding — it's a native addon that must be resolved at runtime
  external: ['@torry/binding'],
});
