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

`build.nu` runs `trunk build --release` then `scripts/seo.nu`, which
writes `robots.txt` and the sitemap index into `dist/`:

| file | content |
|------|---------|
| `robots.txt` | Allow + Sitemap pointer |
| `sitemap.xml` | index |
| `sitemap-core.xml` | landings, regional rankings |
| `sitemap-states.xml` | `/state/{slug}` × all listings (name slugs) |
| `sitemap-tokens.xml` | `/token/{ticker}` |
| `sitemap-corridors-*.xml` | `/from/{slug}/to/{slug}` long-tail (terrestrial pairs) |

nginx must serve these as static files — not the SPA shell. Prefer
`try_files $uri $uri/ /index.html` so real files win.

## SEO surface

- **P0 (done):** crawl foundation — meta shell, robots, full sitemap
  including bilateral corridors (~53k URLs).
- **P1 (next):** prerender HTML bodies for states, tokens, corridors
  so crawlers see text without WASM.
- State URLs use **name slugs**: `/state/japan`, `/state/united-states`
  (legacy `/state/jp` rewrites client-side). Tokens stay tickers: `/token/jpy`.
- Corridor route: `/from/{slug}/to/{slug}` — full terrestrial long-tail
  (~53k ordered pairs, self-pairs omitted).

## layout

- `src/pages/` — home (landings: `/in/:region/by/:field`), tokens, token, country, map
- `states/*.toml` — one file per state: population, land, currency, visa matrix
- `build.rs` — generates `load_countries()` from the toml set
- `scripts/seo.nu` — robots + sitemaps from the toml set
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
