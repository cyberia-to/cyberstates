# cyberstates

Global visa openness analytics — https://cyberstates.net

229 states ranked by FREEDOM (√(eco_out × pop_out)) and OPENNESS
(√(eco_in × pop_in)), with their currency tokens. Leptos CSR + Trunk,
data baked at build time from `states/*.toml`.

## develop

```
PATH="$HOME/.cargo/bin:$PATH" trunk serve        # 127.0.0.1:8080
```

## deploy

```
nu scripts/deploy.nu                 # build + rsync
nu scripts/deploy.nu --release       # + IndexNow (cut a crawl release)
# or step by step:
nu scripts/build.nu
nu scripts/deploy.nu --skip-build
nu scripts/indexnow.nu               # only when cutting a release
```

`build.nu` runs:

1. `trunk build --release` (WASM app)
2. `scripts/seo.nu` → robots + sitemaps
3. `cargo run --features prerender --bin prerender` → crawlable HTML + IndexNow key

`deploy.nu` rsyncs; **IndexNow only with `--release`** (full sitemap, batched).

| file | content |
|------|---------|
| `robots.txt` | Allow + Sitemap pointer |
| `sitemap.xml` | index |
| `sitemap-core.xml` | landings, regional rankings |
| `sitemap-states.xml` | `/state/{slug}` × all listings (name slugs) |
| `sitemap-tokens.xml` | `/token/{ticker}` |
| `sitemap-corridors-*.xml` | `/from/{slug}/to/{slug}` long-tail |
| `state/{slug}/index.html` | prerendered state pages |
| `token/{ticker}/index.html` | prerendered token pages |
| `from/{a}/to/{b}/index.html` | prerendered corridors (~53k) |
| `cyberstates-indexnow-*.txt` | IndexNow key file |

nginx: use `scripts/nginx-cyberstates.net.conf` (entity paths 404 when
missing, no forced trailing slash, includes generated `nginx-redirects.conf`
for `/state/{code}` → `/state/{slug}`, `/country/*` → `/state/*`,
`/methodology` → `/doctrine`).

```
# first-time / nginx changes only:
sudo cp scripts/nginx-cyberstates.net.conf /etc/nginx/sites-available/cyberstates.net
sudo nginx -t && sudo systemctl reload nginx
```

### Search Console / IndexNow

1. [Google Search Console](https://search.google.com/search-console) — add
   `cyberstates.net`, DNS or HTML file verify, submit
   `https://cyberstates.net/sitemap.xml`.
2. **IndexNow is optional** — run when cutting a release
   (`nu scripts/deploy.nu --release` or `nu scripts/indexnow.nu` after
   rsync). Key: `https://cyberstates.net/cyberstates-indexnow-8f3a2c1e9b7d4a60.txt`.
   Full sitemap (~53k URLs) goes in ≤10k batches to IndexNow + Bing.
3. Default share image: `https://cyberstates.net/og.png`

## SEO surface

- **P0 (done):** crawl foundation — meta shell, robots, full sitemap
  including bilateral corridors (~53k URLs).
- **P1 (done):** prerender HTML bodies for states, tokens, corridors
  so crawlers see text without WASM.
- **P2 (done):** JSON-LD `@graph` (WebSite, Organization, WebPage,
  BreadcrumbList, Place / ExchangeRateSpecification), visible breadcrumbs,
  related rankings / corridors internal links.
- **P3 (done):** IndexNow on release cuts (`--release`); enriched entity
  copy + ranking landings; fonts `display=swap`; OG/Twitter on every page.
- State URLs use content `slug`: `/state/japan`, `/state/united-states`.
  Authored in `states/*.toml` (`slug = "…"`). Legacy `/state/jp` still
  resolves and rewrites to the content slug. Tokens stay tickers: `/token/jpy`.
- Corridor route: `/from/{slug}/to/{slug}` — full terrestrial long-tail
  (~53k ordered pairs, self-pairs omitted).

## layout

- `src/pages/` — home (landings: `/in/:region/by/:field`), tokens, token, country, map
- `states/*.toml` — one file per state: name, code, **slug**, population, land, currency, visa matrix
- `build.rs` — generates `load_countries()` from the toml set (reads `slug`, does not invent it)
- `scripts/seo.nu` — robots + sitemaps from content slugs
- `scripts/write_slugs.nu` — fill missing `slug` from name only (never overwrites)
- `scrape_passport.py`, `add_token_prices.py` — data refresh scripts

## roadmap

- **time axis** — stocks gain history; the honest flow is the derivative
  of an audited stock: inflation as emission (the ledger's count of what
  was printed), capital growth as adoption, migration as the tape.
- **rule domains** — border is quoted; token integrity, property rights
  for foreigners, taxation and exit join it, same shape.
- **relational layer** — offshore token holdings, trade, migration flows:
  the kernel is scalar, the world is a graph.
- **refined stocks** — habitable area, age structure.
- **the venue** — quotes become positions; new cyberstates IPO onto the
  table.
