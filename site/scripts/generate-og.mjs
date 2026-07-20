/**
 * Regenerates public/og.png (1200×630) with Instrument Sans.
 * Run from site/:  node scripts/generate-og.mjs
 *
 * Requires: next is installed (uses next/og ImageResponse via a tiny server
 * path is not available offline as a pure script easily). Prefer:
 *   1) temporarily restore opengraph-image.tsx
 *   2) npm run build
 *   3) cp out/opengraph-image public/og.png
 *
 * This file documents the workflow. The committed asset is public/og.png.
 */
console.log(`
OG image workflow (static export + X/Discord crawlers):

  1. Design lives in git history under opengraph-image.tsx commits, or regenerate
     by restoring that file briefly, building, then:
       cp out/opengraph-image public/og.png
  2. Metadata points at https://onca.run/og.png (real .png, image/png).

Why not opengraph-image route?
  output: "export" + trailingSlash serves it as application/octet-stream and
  308-redirects — X shows a blank card. A static public/og.png is reliable.
`);
