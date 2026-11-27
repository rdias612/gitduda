# Website (gitduda.dev.br)

The landing page lives in [`site/`](../site) as plain static files (one HTML
page, a few assets, no build step) and is published to GitHub Pages by
[`.github/workflows/pages.yml`](../.github/workflows/pages.yml) whenever
`site/` changes on `main`. The page reads the latest release from the GitHub
API at load time to fill in version-specific download links, and falls back to
the Releases page if that request fails.

Asset URLs are versioned at deploy time: the workflow runs
[`.github/scripts/version-site-assets.mjs`](../.github/scripts/version-site-assets.mjs),
which appends `?v=<content hash>` to every `assets/…` reference in
`index.html`. Replace an image under the same file name and the deployed page
points at a new URL, so GitHub Pages and browsers (which cache
images for hours) pick it up on the next load. The committed HTML keeps plain
paths, so opening `site/index.html` locally needs no build step.

## One-time setup

1. **Enable Pages from Actions.** Repository *Settings → Pages → Build and
   deployment → Source: GitHub Actions*. Then run the "Deploy Website"
   workflow once (Actions tab → Run workflow) or merge a change under `site/`.
2. **Custom domain.** Still under *Settings → Pages*, set *Custom domain* to
   `gitduda.dev.br` and tick *Enforce HTTPS* once the TLS certificate has been
   issued (`site/CNAME` carries the same value so a deploy never resets it).
3. **DNS for gitduda.dev.br** (apex domain), configured directly in the
   **Registro.br** DNS zone editor:

   | Type  | Name | Value |
   |-------|------|-------|
   | A     | `@`  | `185.199.108.153` |
   | A     | `@`  | `185.199.109.153` |
   | A     | `@`  | `185.199.110.153` |
   | A     | `@`  | `185.199.111.153` |
   | AAAA  | `@`  | `2606:50c0:8000::153` |
   | AAAA  | `@`  | `2606:50c0:8001::153` |
   | AAAA  | `@`  | `2606:50c0:8002::153` |
   | AAAA  | `@`  | `2606:50c0:8003::153` |
   | CNAME | `www` | `rdias612.github.io` |

   Wait for DNS propagation from Registro.br and for GitHub Pages to finish
   provisioning the TLS certificate before ticking *Enforce HTTPS* under
   *Settings → Pages*.

## Updating the page

Edit `site/index.html` and push to `main`. Assets:

- `site/assets/main-window.webp` — hero screenshot, generated from
  `docs/screenshots/main-window.png` (resized to 1600px wide, WebP quality
  82).
- `site/assets/icon-256.png`, `site/assets/icon-512.png`,
  `site/assets/favicon-64.png` — written by `npm run icons:build` from
  `src-tauri/icons/icon-source.png`, alongside the app icons (see
  `src-tauri/icons/README.md`).

Download links are matched by asset-name suffix (`_aarch64.dmg`,
`_x64_en-US.msi`, `_x64-setup.exe`, `_amd64.deb`, `_amd64.AppImage`,
`.x86_64.rpm`); if the bundle naming changes, update the `groups` table in the
page's script alongside `publish-packages.yml`. Once the Microsoft Store
submission is certified, add the official Microsoft Store Web Installer badge
(in *Direct* mode) to `site/index.html` following
[`docs/microsoft-store-web-installer.md`](microsoft-store-web-installer.md).

The `homepage` fields in `src-tauri/tauri.conf.json` (`bundle.homepage`),
`packaging/scoop/gitduda.json`, `packaging/homebrew/gitduda.rb.template`, and
`packaging/winget/rdias612.Gitduda.locale.en-US.yaml.template` (`PublisherUrl`
/ `PackageUrl`) all point at `https://gitduda.dev.br`.
