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
nu scripts/build.nu
rsync -az --delete dist/ cyberproxy:/var/www/html/cyberstates/
```

`build.nu` runs:

1. `trunk build --release` (WASM app)
2. `scripts/seo.nu` → robots + sitemaps
3. `cargo run --features prerender --bin prerender` → crawlable HTML

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

nginx (required for directory index):

```
try_files $uri $uri/ /index.html;
```

Static entity HTML must win over the SPA fallback.

## SEO surface

- **P0 (done):** crawl foundation — meta shell, robots, full sitemap
  including bilateral corridors (~53k URLs).
- **P1 (next):** prerender HTML bodies for states, tokens, corridors
  so crawlers see text without WASM.
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
