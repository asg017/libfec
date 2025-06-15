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
          { text: 'Installing', link: '/getting-started/installing' },
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/asg017/libfec' }
    ]
  }
})
