# cyberstates

Global visa openness analytics — https://cyberstates.net

229 states ranked by FREEDOM (√(eco_out × pop_out)) and OPENNESS
(√(eco_in × pop_in)), with their 155 currency tokens. Leptos CSR + Trunk,
data baked at build time from `states/*.toml`.

## develop

```
PATH="$HOME/.cargo/bin:$PATH" trunk serve        # 127.0.0.1:8080
```

## deploy

```
PATH="$HOME/.cargo/bin:$PATH" trunk build --release
rsync -az --delete dist/ cyberproxy:/var/www/html/cyberstates/
```

## layout

- `src/pages/` — home (landings: `/in/:region/by/:field`), tokens, token, country, map
- `states/*.toml` — one file per state: population, land, currency, visa matrix
- `build.rs` — generates `load_countries()` from the toml set
- `scrape_passport.py`, `add_token_prices.py` — data refresh scripts
