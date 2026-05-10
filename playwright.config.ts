import { defineConfig } from "@playwright/test";

const devServerPort = 4173;
const devServerUrl = `http://127.0.0.1:${devServerPort}`;

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 30000,
  retries: 0,
  webServer: {
    command: `npm run dev:frontend -- --host 127.0.0.1 --port ${devServerPort}`,
    url: devServerUrl,
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
  use: {
    baseURL: devServerUrl,
    headless: true,
  },
});
