# site

The marketing and documentation site for Onca. It lives in the same repo as the
plugins, under `site/`, and deploys on its own.

## Stack

- Next.js 15 (App Router, static export)
- Tailwind CSS v4
- Instrument Sans (`next/font`)
- Motion (available for deliberate animation)

## Develop

Run from this `site/` directory:

```bash
npm install
npm run dev      # http://localhost:3000
npm run build    # static output in out/
```

## Deploy

A push to `main` that touches `site/**` builds this directory and publishes it to
GitHub Pages via `.github/workflows/site.yml` at the repo root. The workflow sets
`NEXT_PUBLIC_BASE_PATH=/onca` so the site works under the Pages path. For a root
deploy (Vercel, or a custom domain), leave that unset.

## Brand

See [brand.md](brand.md). Design follows the
[pols.dev anti-slop law](https://pols.dev/slop).

## License

MIT.
