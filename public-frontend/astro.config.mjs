// @ts-check
import { defineConfig } from "astro/config";
import compressor from "astro-compressor";

// https://astro.build/config
export default defineConfig({
  site: process.env.SITE_URL,
  output: "static",
  trailingSlash: "always",
  base: "/",
  outDir: "../public-assets",
  build: {
    assets: "site-assets",
  },
  server: {
    port: 4321,
  },
  integrations: [compressor({ gzip: true, brotli: false })],
});
