import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    globals: false,
    pool: 'forks',
    poolOptions: {
      forks: {
        execArgv: ['--experimental-wasm-type-reflection'],
      },
    },
  },
});
