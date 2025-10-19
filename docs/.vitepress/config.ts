import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Nylon',
  description: 'High-performance HTTP/HTTPS reverse proxy with plugin support',
  
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
  ],

  themeConfig: {
    logo: '/logo.svg',
    
    nav: [
      { text: 'Guide', link: '/introduction/what-is-nylon' },
      { text: 'Plugins', link: '/plugins/overview' },
      { text: 'API', link: '/api/configuration' },
      { text: 'Examples', link: '/examples/basic-proxy' },
    ],

    sidebar: {
      '/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is Nylon?', link: '/introduction/what-is-nylon' },
            { text: 'Installation', link: '/introduction/installation' },
            { text: 'Quick Start', link: '/introduction/quick-start' },
          ]
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'Configuration', link: '/core/configuration' },
            { text: 'Routing', link: '/core/routing' },
            { text: 'Load Balancing', link: '/core/load-balancing' },
            { text: 'TLS/HTTPS', link: '/core/tls' },
            { text: 'Middleware', link: '/core/middleware' },
          ]
        },
        {
          text: 'Plugin Development',
          items: [
            { text: 'Overview', link: '/plugins/overview' },
            { text: 'Plugin Phases', link: '/plugins/phases' },
            { text: 'Go SDK', link: '/plugins/go-sdk' },
            { text: 'Request Handling', link: '/plugins/request' },
            { text: 'Response Handling', link: '/plugins/response' },
            { text: 'WebSocket Support', link: '/plugins/websocket' },
          ]
        },
        {
          text: 'Examples',
          items: [
            { text: 'Basic Proxy', link: '/examples/basic-proxy' },
            { text: 'Authentication', link: '/examples/authentication' },
            { text: 'Rate Limiting', link: '/examples/rate-limiting' },
            { text: 'Custom Headers', link: '/examples/custom-headers' },
            { text: 'WebSocket Proxy', link: '/examples/websocket' },
          ]
        },
        {
          text: 'API Reference',
          items: [
            { text: 'Configuration Schema', link: '/api/configuration' },
            { text: 'Go SDK API', link: '/api/go-sdk' },
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/AssetsArt/nylon' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2025-present'
    },

    search: {
      provider: 'local'
    }
  }
})

