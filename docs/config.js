const config = {
  gatsby: {
    pathPrefix: '/',
    siteUrl: 'https://docs.shisho.dev',
    gaTrackingId: 'UA-145321226-11',
    trailingSlash: false,
  },
  header: {
    logo: '/images/logo-white.png',
    logoLink: '/',
    title: '',
    githubUrl: 'https://github.com/flatt-security/shisho',
    helpUrl: '',
    tweetText: '',
    social: `<li>
		    <a href="https://twitter.com/flatt_sec_en" target="_blank" rel="noopener">
		      <div class="twitterBtn">
		        <img src='/images/twitter-brands-block.svg' alt={'Twitter'}/>
		      </div>
		    </a>
		  </li>`,
    links: [{ text: '', link: '' }],
    search: {
      enabled: false,
      indexName: '',
      algoliaAppId: process.env.GATSBY_ALGOLIA_APP_ID,
      algoliaSearchKey: process.env.GATSBY_ALGOLIA_SEARCH_KEY,
      algoliaAdminKey: process.env.ALGOLIA_ADMIN_KEY,
    },
  },
  sidebar: {
    forcedNavOrder: ['/', '/shisho', '/shisho-cloud', '/shisho-action', '/shisho-dojo', '/roadmap'],
    collapsedNav: [],
    links: [
      { text: 'Shisho Cloud', link: 'https://shisho.dev' },
      { text: 'Shisho Playground', link: 'https://play.shisho.dev/' },
      { text: 'Shisho Dojo', link: 'https://shisho.dev/dojo/' },
      { text: 'Flatt Security Inc.', link: 'https://flatt.tech/en' },
    ],
    frontline: false,
    ignoreIndex: false,
    title: '',
  },
  siteMetadata: {
    title: 'Shisho Documentation',
    description: 'Shisho Documentation explains the details of usage and benefits and answers any questions regarding Shisho products.',
    ogImage: 'https://docs.shisho.dev/images/ogp.png',
    docsLocation: 'https://github.com/flatt-security/shisho/tree/main/docs/content',
    favicon: 'https://docs.shisho.dev/favicon.png',
  },
  pwa: {
    enabled: false, // disabling this will also remove the existing service worker.
    manifest: {
      name: 'Shisho',
      short_name: 'Shisho',
      start_url: '/',
      background_color: '#032273',
      theme_color: '#032273',
      display: 'standalone',
      crossOrigin: 'use-credentials',
      icons: [
        {
          src: 'src/pwa-512.png',
          sizes: `512x512`,
          type: `image/png`,
        },
      ],
    },
  },
};

module.exports = config;
