import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: 'http://localhost:8080/api/openapi.json',
  output: {
    path: 'src/app/core/api',
  },
  plugins: [
    {
      name: '@hey-api/typescript',
    },
  ],
});
