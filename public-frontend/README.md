# Public Frontend Template

This is the default template for the public-facing website. It displays a welcome page with setup instructions.

## Using Your Own Content

To use your own Astro site instead of this template, set the `PUBLIC_FRONTEND_PATH` environment variable in your `.env` file:

```bash
PUBLIC_FRONTEND_PATH=/path/to/your/astro-site
```

This works for both local development (`make dev`) and Docker builds (`make build`).

## Requirements for Custom Astro Sites

Your custom Astro site must:

1. Be a valid Astro project with `npm run build` script
2. Have its own `package.json` with dependencies
3. **Output to `dist/` directory** (Astro default - remove any custom `outDir` setting)

The build process will:
1. Run `npm run build` in your external site
2. Copy the `dist/` output to `public-assets/`
3. The Rust server serves from `public-assets/`

## Commands (Template Only)

These commands apply to this template directory:

| Command           | Action                                      |
| :---------------- | :------------------------------------------ |
| `npm install`     | Installs dependencies                       |
| `npm run dev`     | Starts local dev server at `localhost:4321` |
| `npm run build`   | Build to `../public-assets/`                |
| `npm run preview` | Preview the build locally                   |
