---
title: Writing docs
sidebar_position: 5
---

# Writing docs

The pages live in `docs/` at the repository root. The Docusaurus site in
`website/` renders them. Only the theme, the landing page, and the build
configuration live in `website/`; all content stays in `docs/`.

## Run the site locally

Install Node 20 or newer. For people who use asdf, `website/.tool-versions`
pins a working version.

```sh
make docs
```

The first run installs the site dependencies. The server starts at
`http://localhost:3000/oink/` and reloads when you change a page in `docs/` or
a file in `website/`.

Other commands:

- `make docs-build` builds the static site into `website/build`.
- `make docs-serve` serves a build at `http://localhost:3000/oink/`.

## Write a page

- Add a Markdown file in `docs/` or in a subfolder. The sidebar is generated
  from the file tree.
- Add front matter with the page title and its order:

  ```markdown
  ---
  title: Page title
  sidebar_position: 2
  ---
  ```

- A folder becomes a sidebar category. `docs/reference/_category_.json` names
  the Reference group.
- Link between pages with relative links that keep the `.md` extension, for
  example `[YAML reference](reference/yaml.md)`. The build fails on a broken
  link.
- Keep pages plain Markdown. Do not add React components and do not import
  theme code.

## Display modes

The site has two display modes. The switch is in the navbar, and the browser
remembers the choice:

- **E-paper (default):** grayscale, paper texture, dithered art.
- **Color:** the same layout with muted red and ochre accents.

The light and dark switch works in both modes. The default is set in
`website/src/plugins/displayModeInit.js`.

## Deployment

GitHub Actions builds the site on pull requests and publishes `main` to
[the GitHub Pages site](https://fabriziosestito.github.io/oink/). The workflow
is `.github/workflows/docs.yml`, and it runs when `docs/`, `website/`, or the
workflow itself changes. In the repository settings, GitHub Pages must use
"GitHub Actions" as the source.

## Other generators

MkDocs and similar tools can read the same Markdown files. Define their
navigation from the [index](index.md); Docusaurus sidebar metadata is optional
for other tools. Keep page links relative when moving content between
generators.

## Maintenance

Update the relevant reference page when changing an exposed function or YAML field.
Document argument types, defaults, returns, errors, and side effects together.
Keep architecture descriptions separate from user instructions and future designs.

The engine tests compile and execute the complete Ink examples in these pages.
The small YAML blocks illustrate individual sections and are not all standalone
rulebooks. The overview includes a complete minimal rulebook and matching story.
