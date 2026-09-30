# oink website

The Docusaurus site that renders the documentation in `../docs/`.

The content lives in `docs/` and is served at the site root. This folder holds
the theme and the build and deployment configuration.

## Commands

Run these from the repository root:

- `make docs` starts the dev server with hot reload at
  <http://localhost:3000/oink/>.
- `make docs-build` builds the static site into `website/build`.
- `make docs-serve` serves a build.

Node 20 or newer is required.

See [docs/publishing.md](../docs/publishing.md) for how to write pages, how
the display modes work, and how deployment works.
