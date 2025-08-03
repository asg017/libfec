import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "libfec",
  description: "Tools to work with campaign finance data from the FEC",
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Examples', link: '/markdown-examples' }
    ],

    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Intro to Campaign Finance', link: '/getting-started/intro-campfin' },
          { text: 'Installation', link: '/getting-started/installation' },
        ]
      },
      {
        text: 'Guides',
        items: [
          { text: 'Contributions & Receipts', link: '/guides/contributions' },
          { text: 'Expenditures & Disbursements', link: '/guides/expenditures' },
          { text: 'Elections', link: '/guides/elections' },
          { text: 'ActBlue & WinRed', link: '/guides/actblue-winred' },
          { text: 'Independent Expenditures', link: '/guides/independent-expenditures' },
        ]
      },
      /*
      {
        text: 'Examples',
        items: [
          { text: 'Elections', link: '#TODO' },
        ]
      },*/
      {
        text: 'Reference',
        items: [
          { text: 'CLI Reference', link: '/reference/cli' },
        ],
        
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/asg017/libfec' }
    ]
  }
})
