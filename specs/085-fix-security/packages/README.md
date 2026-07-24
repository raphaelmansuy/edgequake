# SPEC-085 — Package studies

One study per package (or inseparable stack). Alert IDs live in the [register](../01-alert-register.md).

| Study | Packages | Wave |
|-------|----------|------|
| [PKG-vitest](PKG-vitest.md) | `vitest` | 0 |
| [PKG-next](PKG-next.md) | `next` | 0 |
| [PKG-axios](PKG-axios.md) | `axios` (+ `form-data`) | 1 |
| [PKG-dompurify](PKG-dompurify.md) | `dompurify` | 1 |
| [PKG-astro](PKG-astro.md) | `astro` | 2 |
| [PKG-hono-stack](PKG-hono-stack.md) | `hono`, `@hono/node-server` | 3 |
| [PKG-jackson-databind](PKG-jackson-databind.md) | `jackson-databind` | 4 |
| [PKG-jsonwebtoken](PKG-jsonwebtoken.md) | `jsonwebtoken` | 5 |
| [PKG-opentelemetry-sdk](PKG-opentelemetry-sdk.md) | `opentelemetry_sdk` | 5 |
| [PKG-aws-lc-sys](PKG-aws-lc-sys.md) | `aws-lc-sys` | 5 |
| [PKG-vite](PKG-vite.md) | `vite` | 6 |
| [PKG-postcss](PKG-postcss.md) | `postcss` | 6 |
| [PKG-sharp](PKG-sharp.md) | `sharp` | 6 |
| [PKG-js-yaml](PKG-js-yaml.md) | `js-yaml` | 6 |
| [PKG-transitive-npm](PKG-transitive-npm.md) | brace-expansion, picomatch, minimatch, rollup, flatted, form-data, svgo, fast-uri, esbuild, @babel/core, body-parser, ip-address, smol-toml, yaml | 3+6 |

Template: [`_template-package-study.md`](../_template-package-study.md)
