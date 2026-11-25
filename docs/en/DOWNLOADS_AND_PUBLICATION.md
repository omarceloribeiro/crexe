# Engine downloads and site publication

Status: the counter is implemented; public activation requires a release and deployment. [Português](../DOWNLOADS_E_PUBLICACAO.md).

## What is counted

The landing page shows downloads of the Windows, Linux and macOS archives for the version configured in `src/site/downloads.json`. Each figure comes from the matching GitHub Release asset's `download_count`. The total adds only these packages. Site buttons point to the same assets, so downloads directly through GitHub contribute to the same count.

This is not a count of unique people or installations. It excludes checksums, source ZIPs, examples, documents, other versions and unlisted assets. There is no click counter, visitor identifier, cookie or new engine telemetry. Do not add this total to a separate site click metric: they may overlap.

## Prepare the first publication

1. Validate and merge the selected revision into `main` while the repository is still private.
2. Prepare a tag and draft release corresponding to the tested code; upload the three native packages and their `.sha256` files. Current development packages should be identified as a prerelease, without claiming stability.
3. Make the repository public as decided by the maintainer and publish the release.
4. Check the six URLs without authentication and verify hashes. Release uploads are independent of temporary Actions artifacts.
5. Fill in `downloads.json` with the tag, release URL and actual asset filenames. Platforms without a package keep both fields `null`.
6. Run `npm --prefix src/site ci` and `npm --prefix src/site run check`, commit the configuration and deploy the site.

The build validates the catalog but does not publish releases or check remote URLs. The current CI does not upload assets to Releases automatically. The counter also does not create releases or enable downloads on its own.

## Cloudflare Pages

Use Git integration, production branch `main`, repository root, Node.js 22 or newer, build command `npm --prefix src/site ci && npm --prefix src/site run check`, and output `src/site/dist`. Add `crexe.org` under the project's Custom domains.

The build includes `_worker.js` (Pages Functions advanced mode), `_routes.json` and `_headers` at the output root. Preserve all three for direct upload as well. The Function only runs for `/api/download-counts`; static files continue through the asset service. No D1, KV, GitHub token or secrets are required. A host without Function support can still serve the site and downloads, but not the counts.

After deployment, `/api/download-counts` should return JSON with `schemaVersion: 1`, `status: "available"`, the configured version, total, platform counts and `updatedAt`. Compare with the exact GitHub assets. Check both landing pages, each platform package, checksums, and RFC/white paper downloads. Before a published catalog is configured, `status: "unpublished"` and `total: null` are expected.

## Cache, availability and privacy

The browser only requests the site's own endpoint. The Function queries GitHub's public API without credentials, with a 15-minute edge cache; unavailable responses use a short 60-second cache. The cache is per data center and keyed by the complete catalog configuration, so switching versions cannot reuse the previous version's counts. `updatedAt` records when the figures were retrieved, not the most recent download. Updates are not guaranteed to be real time.

The repository is fixed. Visitor query parameters cannot select a repository, release or external server. The Worker does not forward visitor IPs, cookies or headers to GitHub and does not log personal data. Hosting and download providers retain their own logging policies.

API limits, network errors, private/draft releases, missing assets or inconsistent responses produce an unavailable state without inventing a zero or blocking downloads. Genuine zeros are displayed. Counts cover only public assets in the advertised version, including explicit prereleases; they do not rely on `releases/latest`.

## Maintenance

For each version, create a new tag/release with new files and checksums; verify public downloads; update the catalog; validate and deploy the site. Preserve older releases. The displayed total switches to the new version without accumulating earlier versions. Future release automation should follow the same sequence; it is separate from the counter implementation.

Tests: `npm --prefix src/site run check`. These cover mocked GitHub/cache responses, aggregate integrity, failure states, PT/EN, build and local HTTP. They do not replace public checks after deployment.

## References

- [GitHub: release assets and download_count](https://docs.github.com/en/rest/releases/assets)
- [GitHub: managing releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Cloudflare: Pages Functions advanced mode](https://developers.cloudflare.com/pages/functions/advanced-mode/)
- [Cloudflare: invocation routing with _routes.json](https://developers.cloudflare.com/pages/functions/routing/)
- [Cloudflare: Cache API](https://developers.cloudflare.com/workers/runtime-apis/cache/)
