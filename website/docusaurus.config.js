// @ts-check
import displayModeInit from './src/plugins/displayModeInit.js';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

/**
 * Warm paper syntax highlighting for the color display mode.
 * The e-paper mode desaturates code blocks with a CSS filter.
 */
const paperPrismTheme = {
  plain: {color: '#3a3833', backgroundColor: '#e3dcc9'},
  styles: [
    {
      types: ['comment', 'prolog', 'doctype', 'cdata'],
      style: {color: '#8a8577', fontStyle: 'italic'},
    },
    {types: ['punctuation'], style: {color: '#6e6b62'}},
    {types: ['keyword', 'tag', 'operator'], style: {color: '#8f2f2f'}},
    {types: ['string', 'char', 'attr-value', 'inserted'], style: {color: '#7a6210'}},
    {types: ['number', 'boolean', 'constant', 'symbol'], style: {color: '#466a81'}},
    {types: ['function', 'class-name'], style: {color: '#3f5f4a'}},
    {types: ['attr-name', 'property'], style: {color: '#5a4a6e'}},
    {types: ['deleted'], style: {color: '#8f2f2f', textDecoration: 'line-through'}},
  ],
};

/** Night e-ink syntax highlighting, using the dark half of the Kitten palette. */
const nightPrismTheme = {
  plain: {color: '#c9c9c9', backgroundColor: '#1e1e1e'},
  styles: [
    {
      types: ['comment', 'prolog', 'doctype', 'cdata'],
      style: {color: '#7d7d7d', fontStyle: 'italic'},
    },
    {types: ['punctuation'], style: {color: '#9a9a9a'}},
    {types: ['keyword', 'tag', 'operator'], style: {color: '#d98c8c'}},
    {types: ['string', 'char', 'attr-value', 'inserted'], style: {color: '#d4b45a'}},
    {types: ['number', 'boolean', 'constant', 'symbol'], style: {color: '#8DA8B9'}},
    {types: ['function', 'class-name'], style: {color: '#8bc48f'}},
    {types: ['attr-name', 'property'], style: {color: '#c8b9cb'}},
    {types: ['deleted'], style: {color: '#d98c8c', textDecoration: 'line-through'}},
  ],
};

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'oink',
  tagline: 'Gamebooks for e-ink',
  favicon: 'img/favicon.png',

  // Future flags, see https://docusaurus.io/docs/api/docusaurus-config#future
  future: {
    v4: true, // Improve compatibility with the upcoming Docusaurus v4
  },

  url: 'https://fabriziosestito.github.io',
  baseUrl: '/oink/',

  // GitHub Pages deployment config.
  organizationName: 'fabriziosestito',
  projectName: 'oink',
  trailingSlash: false,

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          path: '../docs',
          routeBasePath: 'docs',
          sidebarPath: './sidebars.js',
          editUrl: 'https://github.com/fabriziosestito/oink/edit/main/docs/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  plugins: [displayModeInit],

  themes: [
    [
      '@easyops-cn/docusaurus-search-local',
      {
        hashed: true,
        language: ['en'],
        docsDir: '../docs',
        docsRouteBasePath: '/docs',
        indexBlog: false,
        highlightSearchTermsOnTargetPage: true,
      },
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/social-preview.png',
      colorMode: {
        defaultMode: 'light',
        disableSwitch: false,
        respectPrefersColorScheme: false,
      },
      navbar: {
        title: 'oink',
        logo: {
          alt: 'A pig knight reading a large red book',
          src: 'img/oink-logo.png',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Docs',
          },
          {type: 'custom-displayMode', position: 'right'},
          {type: 'search', position: 'right'},
          {
            href: 'https://github.com/fabriziosestito/oink',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'light',
        logo: {
          alt: 'A pig knight reading a large red book',
          src: 'img/oink-logo.png',
          width: 64,
        },
        links: [
          {
            title: 'Documentation',
            items: [
              {label: 'Introduction', to: '/docs'},
              {label: 'Rulebook', to: '/docs/rulebook'},
              {label: 'Architecture', to: '/docs/architecture'},
            ],
          },
          {
            title: 'Reference',
            items: [
              {label: 'YAML', to: '/docs/reference/yaml'},
              {label: 'Checks and modifiers', to: '/docs/reference/checks'},
              {label: 'Ink API', to: '/docs/reference/ink-api'},
            ],
          },
          {
            title: 'Project',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/fabriziosestito/oink',
              },
              {
                label: 'Issues',
                href: 'https://github.com/fabriziosestito/oink/issues',
              },
              {
                label: 'License',
                href: 'https://github.com/fabriziosestito/oink/blob/main/LICENSE',
              },
            ],
          },
        ],
        copyright: `oink is Apache-2.0 licensed. Built with Docusaurus.`,
      },
      docs: {
        sidebar: {
          hideable: false,
          autoCollapseCategories: true,
        },
      },
      tableOfContents: {
        minHeadingLevel: 2,
        maxHeadingLevel: 3,
      },
      prism: {
        theme: paperPrismTheme,
        darkTheme: nightPrismTheme,
      },
    }),
};

export default config;
