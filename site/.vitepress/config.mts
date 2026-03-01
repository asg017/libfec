import { defineConfig } from 'vitepress'
import llmstxt, {copyOrDownloadAsMarkdownButtons } from 'vitepress-plugin-llms'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "libfec",
  description: "Tools to work with campaign finance data from the FEC",
  appearance: false,
  base: "/libfec/",
  vite: {
    plugins: [
      llmstxt()
    ]
  },
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Examples', link: '/examples' }
    ],

    sidebar: [
      {
        text: 'Getting Started',
        items: [
          /*{ text: 'Intro to Campaign Finance', link: '/getting-started/intro-campfin' },*/
          { text: 'Installation', link: '/getting-started/installation' },
          { text: 'Basic Usage', link: '/getting-started/basic-usage' },
        ]
      },
      /*
      {
        text: 'Examples',
        items: [
          {text: "Filing Deadlines", link: '/examples/filing-deadlines'},
        ],
      },*/
      {
        text: 'Guides',
        items: [
          /*
          { text: 'Contributions & Receipts', link: '/guides/contributions' },
          { text: 'Expenditures & Disbursements', link: '/guides/expenditures' },
          { text: 'Elections', link: '/guides/elections' },
          { text: 'ActBlue & WinRed', link: '/guides/actblue-winred' },
          { text: 'Independent Expenditures', link: '/guides/independent-expenditures' },
          */
          //{ text: 'Campaigns', link: '/guides/campaigns' },
          { text: 'Caching', link: '/guides/cache' },
          { text: 'FastFEC Compatibility', link: '/guides/fastfec' },
        ]
      },
      {
        text: 'Reference',
        items: [
          { text: 'CLI Reference', link: '/reference/cli' },
          { text: 'SQL Reference', link: '/reference/sql' },
        ],
        
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/asg017/libfec' }
    ]
  }
})
