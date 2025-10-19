# Nylon Documentation

This directory contains the documentation for Nylon Reverse Proxy, built with [VitePress](https://vitepress.dev/).

## Development

```bash
# Install dependencies
bun install

# Start dev server
bun run dev

# Build for production
bun run build

# Preview production build
bun run preview
```

## Structure

```
docs/
├── .vitepress/          # VitePress configuration
│   ├── config.ts        # Site configuration
│   └── theme/           # Custom theme
├── introduction/        # Getting started guides
├── core/                # Core concepts
├── plugins/             # Plugin development
├── examples/            # Usage examples
└── api/                 # API reference
```

## Writing Documentation

- Use Markdown (.md) files
- Follow the existing structure
- Add new pages to `.vitepress/config.ts` sidebar
- Use code blocks with language hints
- Add examples where appropriate

## Deployment

The documentation can be deployed to any static hosting service:

- GitHub Pages
- Netlify
- Vercel
- Cloudflare Pages

Build the site with `npm run build` and deploy the `.vitepress/dist` directory.

