# SPEC-008-08: Deployment Strategy

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document specifies the deployment strategy for the unified Astro + Starlight site. The site is fully static (zero server-side runtime), enabling deployment to any static hosting provider with a global CDN.

---

## 2. Deployment Architecture

```
+----------------+       +-------------------+       +------------------+
| GitHub Repo    |       | CI Pipeline       |       | CDN / Hosting    |
| main branch    |------>| (GitHub Actions)  |------>| (Static Files)   |
|                |       |                   |       |                  |
| docs/          |       | pnpm install      |       | dist/            |
| edgequake-     |       | prebuild-validate |       |  index.html      |
|   website/     |       | astro build       |       |  docs/           |
+----------------+       | verify output     |       |  pagefind/       |
                         | deploy            |       |  sitemap-*.xml   |
                         +-------------------+       +--------+---------+
                                                              |
                                                     +--------v---------+
                                                     | edgequake.com    |
                                                     | (Custom Domain)  |
                                                     +------------------+
```

---

## 3. Hosting Provider Options

### 3.1 Recommended: Cloudflare Pages

| Aspect              | Detail                                                     |
| ------------------- | ---------------------------------------------------------- |
| Free tier           | 500 builds/month, unlimited bandwidth, unlimited sites     |
| Build               | Via GitHub Actions (push dist/) or native Cloudflare build |
| CDN                 | Global edge network (300+ locations)                       |
| Custom domain       | CNAME or proxied A record                                  |
| Preview deployments | Auto-generated for each PR                                 |
| HTTPS               | Automatic, free SSL/TLS                                    |
| Redirect rules      | `_redirects` file or `_headers` file                       |

#### Cloudflare Pages Setup

```bash
# 1. Connect GitHub repo to Cloudflare Pages
#    Dashboard → Pages → Create a project → Connect to Git

# 2. Or deploy via GitHub Actions (see 05-build-pipeline.md):
wrangler pages deploy edgequake-website/dist --project-name=edgequake
```

### 3.2 Alternative: GitHub Pages

| Aspect              | Detail                                                   |
| ------------------- | -------------------------------------------------------- |
| Free tier           | Unlimited for public repos                               |
| Build               | Via GitHub Actions (push to gh-pages branch or artifact) |
| CDN                 | GitHub's Fastly CDN                                      |
| Custom domain       | CNAME file in repo root                                  |
| Preview deployments | Not built-in (needs separate workflow)                   |
| HTTPS               | Automatic with custom domain                             |

#### GitHub Pages Setup

```yaml
# In .github/workflows/deploy-website.yml (deploy job)
- name: Deploy to GitHub Pages
  uses: actions/deploy-pages@v4
```

### 3.3 Alternative: Vercel

| Aspect              | Detail                                |
| ------------------- | ------------------------------------- |
| Free tier           | Hobby plan, 100 GB bandwidth/month    |
| Build               | Native Astro adapter or static output |
| CDN                 | Vercel Edge Network                   |
| Custom domain       | Dashboard configuration               |
| Preview deployments | Automatic per PR                      |

---

## 4. Domain Configuration

### 4.1 Custom Domain: edgequake.com

```
edgequake.com  ──>  CNAME ──>  edgequake.pages.dev  (Cloudflare Pages)
                    or
                    CNAME ──>  <user>.github.io      (GitHub Pages)
```

### 4.2 CNAME File

Already exists at `edgequake-website/public/CNAME`:

```
edgequake.com
```

This file is copied to `dist/` during build, ensuring the hosting provider recognizes the custom domain.

### 4.3 DNS Records

| Record | Name            | Value                | TTL  |
| ------ | --------------- | -------------------- | ---- |
| CNAME  | `edgequake.com` | Hosting provider URL | Auto |
| CNAME  | `www`           | `edgequake.com`      | Auto |

---

## 5. Redirect Rules

### 5.1 www → apex

```
# public/_redirects (Cloudflare Pages / Netlify format)
https://www.edgequake.com/* https://edgequake.com/:splat 301
```

### 5.2 Legacy URL Redirects

After migrating from Next.js, preserve any indexed URLs:

```
# Next.js used trailing slashes; Astro does too by default
# No redirects needed for matching path structure

# If docs were previously at a different path:
# /documentation/* /docs/:splat 301
```

### 5.3 Trailing Slashes

Astro config:

```javascript
// astro.config.mjs
export default defineConfig({
  trailingSlash: "always", // Matches Next.js behavior
});
```

---

## 6. CDN & Caching

### 6.1 Cache Headers

```
# public/_headers
/*
  Cache-Control: public, max-age=0, must-revalidate

/pagefind/*
  Cache-Control: public, max-age=31536000, immutable

/_astro/*
  Cache-Control: public, max-age=31536000, immutable
```

Explanation:

- HTML pages: Always revalidate (ensures fresh content on deploy)
- Static assets (`/_astro/` hashed files, Pagefind index): Cache forever (content-hashed filenames)

### 6.2 Asset Fingerprinting

Astro automatically fingerprints built assets:

```
dist/_astro/
  index.DqX4b2c.css
  GraphAnimation.B7kp9.js
```

These hashed filenames enable aggressive caching.

---

## 7. Preview Deployments

For PRs touching `docs/` or `edgequake-website/`:

| Provider         | Preview URL                                         |
| ---------------- | --------------------------------------------------- |
| Cloudflare Pages | `https://<commit>.edgequake.pages.dev`              |
| Vercel           | `https://edgequake-<hash>.vercel.app`               |
| GitHub Pages     | (Requires custom workflow to deploy-preview branch) |

Preview deployments enable reviewing docs changes before merging.

---

## 8. Rollback Strategy

Since the site is fully static:

1. **Revert commit** — Triggers new build from previous good state
2. **Redeploy previous build** — Cloudflare Pages keeps build history
3. **Instant rollback** — Cloudflare/Vercel allow one-click rollback to any previous deployment

No database, no server state, no migration — rollback is always safe and instant.

---

## 9. Monitoring

| What         | Tool                                        |
| ------------ | ------------------------------------------- |
| Uptime       | Cloudflare Analytics or UptimeRobot (free)  |
| Performance  | Lighthouse CI in GitHub Actions             |
| Errors       | Cloudflare Analytics (4xx/5xx)              |
| Search usage | Pagefind analytics (client-side, if added)  |
| Traffic      | Cloudflare Web Analytics (privacy-friendly) |

---

## 10. Cross-References

- [00-overview.md](./00-overview.md) — Goal G8: single deployment point
- [05-build-pipeline.md](./05-build-pipeline.md) — CI/CD workflow and deploy step
- [06-search-navigation-seo.md](./06-search-navigation-seo.md) — Sitemap, robots.txt, CDN caching interaction
- [09-migration-roadmap.md](./09-migration-roadmap.md) — DNS cutover timing in Phase 4
