//! Build-time HTML prerender for SEO.
//!
//! Reads `states/*.toml` (slug is content) + trunk `dist/index.html` shell,
//! writes crawlable pages under dist/:
//!   state/{slug}/index.html
//!   token/{ticker}/index.html
//!   from/{a}/to/{b}/index.html
//!   doctrine/index.html, tokens/index.html, key landings
//!
//!   cargo run --release --bin prerender -- dist

use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const BASE: &str = "https://cyberstates.net";
const AGGREGATES: &[&str] = &["OCNA", "AFRI", "EURA", "AMER"];

#[derive(Debug, Clone, Deserialize)]
struct VisaAccess {
    country: String,
    #[serde(rename = "type")]
    access_type: String,
    #[serde(default)]
    days: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct StateToml {
    name: String,
    code: String,
    slug: String,
    flag: String,
    region: String,
    population: u64,
    land_area_km2: u64,
    currency_code: String,
    currency_name: String,
    money_supply_b_usd: f64,
    #[serde(default)]
    token_price_usd: f64,
    visa_free_destinations: u32,
    visa_free_inbound: u32,
    #[serde(default)]
    oceans: String,
    #[serde(default)]
    visa_access: Vec<VisaAccess>,
}

#[derive(Clone)]
struct State {
    t: StateToml,
    rank_capital: usize,
    rank_pop: usize,
    rank_area: usize,
    rank_freedom: usize,     // by visa_free_destinations
    rank_hospitality: usize, // by visa_free_inbound
}

#[derive(Clone)]
struct TokenAgg {
    code: String,
    name: String,
    price_usd: f64,
    total_cap_b: f64,
    states: Vec<(String, String, String)>, // slug, name, flag
}

fn main() {
    let t0 = Instant::now();
    let dist = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist"));
    let states_dir = PathBuf::from("states");

    if !dist.join("index.html").exists() {
        eprintln!(
            "error: {}/index.html missing — run trunk build first",
            dist.display()
        );
        std::process::exit(1);
    }
    if !states_dir.exists() {
        eprintln!("error: states/ not found — run from repo root");
        std::process::exit(1);
    }

    let shell = fs::read_to_string(dist.join("index.html")).expect("read index.html");
    let head_assets = extract_head_assets(&shell);

    let mut raw = load_states(&states_dir);
    raw.sort_by(|a, b| a.slug.cmp(&b.slug));

    let states = rank_states(raw);
    let by_name: HashMap<String, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.t.name.clone(), i))
        .collect();
    let by_slug: HashMap<String, usize> = states
        .iter()
        .enumerate()
        .map(|(i, s)| (s.t.slug.clone(), i))
        .collect();
    let tokens = aggregate_tokens(&states);

    let mut n_pages = 0usize;

    // --- states ---
    for s in &states {
        let path = dist.join("state").join(&s.t.slug).join("index.html");
        let html = page_state(s, &states, &head_assets);
        write_page(&path, &html);
        n_pages += 1;
    }
    eprintln!("  states:     {}", states.len());

    // --- tokens ---
    for tok in &tokens {
        let slug = tok.code.to_lowercase();
        let path = dist.join("token").join(&slug).join("index.html");
        let html = page_token(tok, &head_assets);
        write_page(&path, &html);
        n_pages += 1;
    }
    eprintln!("  tokens:     {}", tokens.len());

    // --- corridors (terrestrial non-aggregate ordered pairs) ---
    let corridor: Vec<&State> = states.iter().filter(|s| is_corridor_eligible(s)).collect();
    let mut n_corr = 0usize;
    for (i, a) in corridor.iter().enumerate() {
        for (j, b) in corridor.iter().enumerate() {
            if i == j {
                continue;
            }
            let path = dist
                .join("from")
                .join(&a.t.slug)
                .join("to")
                .join(&b.t.slug)
                .join("index.html");
            let html = page_corridor(a, b, &head_assets);
            write_page(&path, &html);
            n_corr += 1;
        }
        if (i + 1) % 50 == 0 {
            eprint!(
                "\r  corridors:  {} / ~{}",
                n_corr,
                corridor.len() * (corridor.len() - 1)
            );
        }
    }
    n_pages += n_corr;
    eprintln!("\r  corridors:  {}                    ", n_corr);

    // --- static landings ---
    for (rel, html) in landing_pages(&states, &tokens, &head_assets) {
        let path = if rel == "/" {
            // keep trunk index; only enrich if needed — skip overwrite of root shell assets
            continue;
        } else {
            dist.join(rel.trim_start_matches('/')).join("index.html")
        };
        write_page(&path, &html);
        n_pages += 1;
    }
    // doctrine, tokens list, freedom ranking, hospitality, listing
    write_page(
        &dist.join("doctrine").join("index.html"),
        &page_simple(
            "Doctrine — the sovereignty terminal | Cyberstates",
            "A state is territory + population + rules. Token is the deepest rule. How cyberstates ranks capital, freedom and hospitality.",
            "/doctrine",
            &doctrine_body(),
            &head_assets,
        ),
    );
    n_pages += 1;
    write_page(
        &dist.join("tokens").join("index.html"),
        &page_simple(
            "Currency tokens of all states | Cyberstates",
            "Every state token ranked by capital stock — money the world agrees to hold.",
            "/tokens",
            &tokens_index_body(&tokens),
            &head_assets,
        ),
    );
    n_pages += 1;
    write_page(
        &dist.join("by").join("travel-freedom").join("index.html"),
        &page_simple(
            "States by travel freedom | Cyberstates",
            "States ranked by outbound visa access — where citizens can travel.",
            "/by/travel-freedom",
            &ranking_body(
                "Travel freedom",
                &states,
                |s| s.rank_freedom,
                |s| format!("{} visa-free destinations", s.t.visa_free_destinations),
            ),
            &head_assets,
        ),
    );
    n_pages += 1;
    write_page(
        &dist.join("by").join("hospitality").join("index.html"),
        &page_simple(
            "States by hospitality | Cyberstates",
            "States ranked by inbound openness — who can visit.",
            "/by/hospitality",
            &ranking_body(
                "Hospitality",
                &states,
                |s| s.rank_hospitality,
                |s| format!("{} visa-free inbound", s.t.visa_free_inbound),
            ),
            &head_assets,
        ),
    );
    n_pages += 1;
    write_page(
        &dist.join("by").join("capital").join("index.html"),
        &page_simple(
            "States by capital stock | Cyberstates",
            "States ranked by money stock — the market verdict on the rule-set.",
            "/by/capital",
            &ranking_body(
                "Capital",
                &states,
                |s| s.rank_capital,
                |s| format!("${:.0}B money stock", s.t.money_supply_b_usd),
            ),
            &head_assets,
        ),
    );
    n_pages += 1;

    // Enrich root index body with a crawlable state directory (keep trunk head)
    let root_body = home_body(&states);
    let root_html = inject_body_into_shell(&shell, &root_body);
    fs::write(dist.join("index.html"), root_html).expect("write root index");

    let _ = (&by_name, &by_slug); // reserved for future cross-links density
    eprintln!(
        "prerender: {} pages in {:.1}s → {}",
        n_pages,
        t0.elapsed().as_secs_f64(),
        dist.display()
    );
}

fn load_states(dir: &Path) -> Vec<StateToml> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "toml").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let text = fs::read_to_string(e.path()).unwrap();
        match toml::from_str::<StateToml>(&text) {
            Ok(s) => {
                if s.slug.is_empty() {
                    panic!("empty slug in {:?}", e.path());
                }
                out.push(s);
            }
            Err(err) => eprintln!("warn: skip {:?}: {}", e.path(), err),
        }
    }
    out
}

fn rank_states(raw: Vec<StateToml>) -> Vec<State> {
    let n = raw.len();
    let mut idx: Vec<usize> = (0..n).collect();

    let mut rank_capital = vec![0; n];
    let mut rank_pop = vec![0; n];
    let mut rank_area = vec![0; n];
    let mut rank_freedom = vec![0; n];
    let mut rank_hospitality = vec![0; n];

    let assign =
        |rank: &mut [usize], idx: &mut [usize], raw: &[StateToml], key: fn(&StateToml) -> f64| {
            idx.sort_by(|&a, &b| {
                key(&raw[b])
                    .partial_cmp(&key(&raw[a]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (r, &i) in idx.iter().enumerate() {
                rank[i] = r + 1;
            }
        };

    assign(&mut rank_capital, &mut idx, &raw, |s| s.money_supply_b_usd);
    assign(&mut rank_pop, &mut idx, &raw, |s| s.population as f64);
    assign(&mut rank_area, &mut idx, &raw, |s| s.land_area_km2 as f64);
    assign(&mut rank_freedom, &mut idx, &raw, |s| {
        s.visa_free_destinations as f64
    });
    assign(&mut rank_hospitality, &mut idx, &raw, |s| {
        s.visa_free_inbound as f64
    });

    raw.into_iter()
        .enumerate()
        .map(|(i, t)| State {
            t,
            rank_capital: rank_capital[i],
            rank_pop: rank_pop[i],
            rank_area: rank_area[i],
            rank_freedom: rank_freedom[i],
            rank_hospitality: rank_hospitality[i],
        })
        .collect()
}

fn is_corridor_eligible(s: &State) -> bool {
    !AGGREGATES.contains(&s.t.code.as_str())
        && !matches!(
            s.t.region.as_str(),
            "Oceans" | "Terra Nullius" | "Solar System"
        )
}

fn aggregate_tokens(states: &[State]) -> Vec<TokenAgg> {
    let mut map: HashMap<String, TokenAgg> = HashMap::new();
    for s in states {
        let code = s.t.currency_code.clone();
        if code.is_empty() {
            continue;
        }
        let e = map.entry(code.clone()).or_insert_with(|| TokenAgg {
            code: code.clone(),
            name: s.t.currency_name.clone(),
            price_usd: s.t.token_price_usd,
            total_cap_b: 0.0,
            states: Vec::new(),
        });
        e.total_cap_b += s.t.money_supply_b_usd;
        if s.t.token_price_usd > 0.0 {
            e.price_usd = s.t.token_price_usd;
        }
        if e.name.is_empty() {
            e.name = s.t.currency_name.clone();
        }
        e.states
            .push((s.t.slug.clone(), s.t.name.clone(), s.t.flag.clone()));
    }
    let mut v: Vec<_> = map.into_values().collect();
    v.sort_by(|a, b| {
        b.total_cap_b
            .partial_cmp(&a.total_cap_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for t in &mut v {
        t.states.sort_by(|a, b| a.1.cmp(&b.1));
    }
    v
}

fn access_label(t: &str) -> &str {
    match t {
        "visa-free" => "visa-free",
        "visa-on-arrival" => "visa on arrival",
        "eta" | "e-visa" => "online (e-visa/eta)",
        "visa-required" => "visa required",
        "no-admission" => "no admission",
        _ => "unknown",
    }
}

fn find_access(from: &State, to: &State) -> (String, Option<u32>) {
    for e in &from.t.visa_access {
        if e.country == to.t.name {
            return (e.access_type.clone(), e.days);
        }
    }
    ("unknown".into(), None)
}

// --- HTML helpers ---

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape for JSON string literals inside application/ld+json.
fn jesc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn extract_head_assets(shell: &str) -> String {
    // Continuous slice: fonts/preconnect through hashed css + wasm bootstrap.
    let start_markers = [
        "rel=\"preconnect\"",
        "fonts.googleapis.com",
        "rel=\"stylesheet\"",
    ];
    let start = start_markers
        .iter()
        .filter_map(|m| shell.find(m))
        .min()
        .unwrap_or_else(|| shell.find("<head>").unwrap_or(0));
    // walk back to line start for clean tags
    let start = shell[..start].rfind('\n').map(|i| i + 1).unwrap_or(start);
    let end = shell.find("</head>").unwrap_or(shell.len());
    let chunk = shell[start..end].trim();
    // Drop trunk-only comments if any; keep link/script/preload as emitted by trunk.
    chunk.to_string()
}

fn write_page(path: &Path, html: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, html).unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

fn shell_page(
    title: &str,
    description: &str,
    canonical_path: &str,
    json_ld: &str,
    body_inner: &str,
    head_assets: &str,
) -> String {
    let canon = format!("{}{}", BASE, canonical_path);
    let title_e = esc(title);
    let desc_e = esc(description);
    let canon_e = esc(&canon);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>{title}</title>
<meta name="description" content="{desc}" />
<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1" />
<link rel="canonical" href="{canon}" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="Cyberstates" />
<meta property="og:url" content="{canon}" />
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{desc}" />
<meta property="og:locale" content="en_US" />
<meta name="twitter:card" content="summary" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{desc}" />
<meta name="theme-color" content="#000000" />
<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
{assets}
<script type="application/ld+json">{ld}</script>
<style>
  .seo-crawl {{ font-family: system-ui, sans-serif; max-width: 42rem; margin: 1.5rem auto; padding: 0 1rem 4rem; color: #e8e8e8; background: #000; line-height: 1.55; }}
  .seo-crawl a {{ color: #4ade80; }}
  .seo-crawl h1 {{ font-size: 1.75rem; color: #fff; margin: 0.5rem 0 1rem; }}
  .seo-crawl h2 {{ font-size: 1rem; letter-spacing: 0.12em; text-transform: uppercase; color: #666; margin: 1.75rem 0 0.75rem; }}
  .seo-crawl table {{ width: 100%; border-collapse: collapse; font-size: 0.9rem; }}
  .seo-crawl th, .seo-crawl td {{ text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #222; }}
  .seo-crawl .muted {{ color: #666; }}
  .seo-crawl ul {{ padding-left: 1.2rem; }}
  .seo-crawl .nav {{ font-size: 0.85rem; margin-bottom: 1rem; }}
  .seo-crawl .nav a {{ margin-right: 0.75rem; }}
</style>
</head>
<body>
<main class="seo-crawl" id="seo-static">
{body}
</main>
</body>
</html>
"##,
        title = title_e,
        desc = desc_e,
        canon = canon_e,
        assets = head_assets,
        ld = json_ld,
        body = body_inner,
    )
}

fn page_simple(title: &str, desc: &str, path: &str, body: &str, assets: &str) -> String {
    let ld = format!(
        r#"{{"@context":"https://schema.org","@type":"WebPage","name":"{}","url":"{}{}","description":"{}"}}"#,
        jesc(title),
        BASE,
        path,
        jesc(desc)
    );
    shell_page(title, desc, path, &ld, body, assets)
}

fn page_state(s: &State, all: &[State], assets: &str) -> String {
    let path = format!("/state/{}", s.t.slug);
    let title = format!(
        "{} — capital #{}, freedom #{} | Cyberstates",
        s.t.name, s.rank_capital, s.rank_freedom
    );
    let desc = format!(
        "{} ({}): money stock ${:.0}B (rank #{}), population {}, land {} km². Token {}. Visa-free destinations: {}. Hospitality inbound: {}.",
        s.t.name,
        s.t.code,
        s.t.money_supply_b_usd,
        s.rank_capital,
        s.t.population,
        s.t.land_area_km2,
        s.t.currency_code,
        s.t.visa_free_destinations,
        s.t.visa_free_inbound
    );
    let ld = format!(
        r#"{{"@context":"https://schema.org","@type":"Place","name":"{}","identifier":"{}","url":"{}{}","additionalProperty":[{{"@type":"PropertyValue","name":"capital_b_usd","value":{}}},{{"@type":"PropertyValue","name":"population","value":{}}},{{"@type":"PropertyValue","name":"currency","value":"{}"}}]}}"#,
        jesc(&s.t.name),
        jesc(&s.t.code),
        BASE,
        path,
        s.t.money_supply_b_usd,
        s.t.population,
        jesc(&s.t.currency_code)
    );

    // Top outbound visa-free destinations (link when we know the slug)
    let name_to_slug: HashMap<&str, &str> = all
        .iter()
        .map(|x| (x.t.name.as_str(), x.t.slug.as_str()))
        .collect();
    let mut free: Vec<&VisaAccess> =
        s.t.visa_access
            .iter()
            .filter(|e| e.access_type == "visa-free")
            .collect();
    free.sort_by(|a, b| a.country.cmp(&b.country));
    let free_links: String = free
        .iter()
        .take(40)
        .map(|e| {
            if let Some(slug) = name_to_slug.get(e.country.as_str()) {
                format!(
                    r#"<li><a href="/state/{slug}">{name}</a> — visa-free{days}</li>"#,
                    slug = slug,
                    name = esc(&e.country),
                    days = e.days.map(|d| format!(" ({d} days)")).unwrap_or_default()
                )
            } else {
                format!("<li>{} — visa-free</li>", esc(&e.country))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Sample corridors from this state to top capital peers
    let mut peers: Vec<&State> = all
        .iter()
        .filter(|o| o.t.slug != s.t.slug && is_corridor_eligible(o))
        .collect();
    peers.sort_by_key(|o| o.rank_capital);
    let corr_links: String = peers
        .iter()
        .take(12)
        .map(|o| {
            format!(
                r#"<li><a href="/from/{a}/to/{b}">{an} → {bn}</a></li>"#,
                a = s.t.slug,
                b = o.t.slug,
                an = esc(&s.t.name),
                bn = esc(&o.t.name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let body = format!(
        r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">Tokens</a><a href="/by/travel-freedom">Freedom</a><a href="/doctrine">Doctrine</a></nav>
<p class="muted">{flag} {region} · {code}</p>
<h1>{name}</h1>
<p>{name} is listed as a cyberstate — territory, population and rules measured on the sovereignty terminal. Token <a href="/token/{tok}">{tok}</a> ({tokn}).</p>
<h2>Fundamentals</h2>
<table>
<tr><th>Capital (money stock)</th><td>${cap:.1}B · rank #{rc} of {n}</td></tr>
<tr><th>Population</th><td>{pop} · rank #{rp}</td></tr>
<tr><th>Territory</th><td>{area} km² · rank #{ra}</td></tr>
<tr><th>Token price</th><td>{price}</td></tr>
<tr><th>Travel freedom</th><td>{vf} visa-free destinations · rank #{rf}</td></tr>
<tr><th>Hospitality</th><td>{vi} visa-free inbound · rank #{rh}</td></tr>
</table>
<h2>Visa-free destinations</h2>
<ul>{free}</ul>
{more_free}
<h2>Corridors from {name}</h2>
<ul>{corr}</ul>
<p class="muted"><a href="/from/{slug}/to/japan">Sample corridor builder</a> — full long-tail at /from/{slug}/to/…</p>
"##,
        flag = s.t.flag,
        region = esc(&s.t.region),
        code = esc(&s.t.code),
        name = esc(&s.t.name),
        tok = s.t.currency_code.to_lowercase(),
        tokn = esc(&s.t.currency_name),
        cap = s.t.money_supply_b_usd,
        rc = s.rank_capital,
        n = all.len(),
        pop = s.t.population,
        rp = s.rank_pop,
        area = s.t.land_area_km2,
        ra = s.rank_area,
        price = if s.t.token_price_usd > 0.0 {
            format!("${:.6}", s.t.token_price_usd)
        } else {
            "N/A".into()
        },
        vf = s.t.visa_free_destinations,
        rf = s.rank_freedom,
        vi = s.t.visa_free_inbound,
        rh = s.rank_hospitality,
        free = free_links,
        more_free = if free.len() > 40 {
            format!(
                "<p class=\"muted\">…and {} more in the live terminal.</p>",
                free.len() - 40
            )
        } else {
            String::new()
        },
        corr = corr_links,
        slug = s.t.slug,
    );
    shell_page(&title, &desc, &path, &ld, &body, assets)
}

fn page_token(t: &TokenAgg, assets: &str) -> String {
    let slug = t.code.to_lowercase();
    let path = format!("/token/{}", slug);
    let title = format!(
        "{} ({}) — states on this token | Cyberstates",
        t.name, t.code
    );
    let desc = format!(
        "{} money stock ${:.0}B across {} states. Price ${:.6}.",
        t.code,
        t.total_cap_b,
        t.states.len(),
        t.price_usd
    );
    let ld = format!(
        r#"{{"@context":"https://schema.org","@type":"ExchangeRateSpecification","currency":"{}","url":"{}{}","name":"{}"}}"#,
        jesc(&t.code),
        BASE,
        path,
        jesc(&t.name)
    );
    let rows: String = t
        .states
        .iter()
        .map(|(s, n, f)| {
            format!(
                r#"<li>{f} <a href="/state/{s}">{n}</a></li>"#,
                f = f,
                s = s,
                n = esc(n)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">All tokens</a><a href="/doctrine">Doctrine</a></nav>
<h1>{code} — {name}</h1>
<p>Token of record for {n} cyberstates. Aggregate capital ${cap:.1}B. Price {price}.</p>
<h2>States on {code}</h2>
<ul>{rows}</ul>
"##,
        code = esc(&t.code),
        name = esc(&t.name),
        n = t.states.len(),
        cap = t.total_cap_b,
        price = if t.price_usd > 0.0 {
            format!("${:.6}", t.price_usd)
        } else {
            "N/A".into()
        },
        rows = rows,
    );
    shell_page(&title, &desc, &path, &ld, &body, assets)
}

fn page_corridor(a: &State, b: &State, assets: &str) -> String {
    let path = format!("/from/{}/to/{}", a.t.slug, b.t.slug);
    let (out_t, out_d) = find_access(a, b);
    let (in_t, in_d) = find_access(b, a);
    let out_l = access_label(&out_t);
    let in_l = access_label(&in_t);
    let title = format!(
        "{} → {} visa corridor ({}) | Cyberstates",
        a.t.name, b.t.name, out_l
    );
    let desc = format!(
        "Can {} passport holders enter {}? {}. Reverse: {}. Capital ranks #{} → #{}.",
        a.t.name, b.t.name, out_l, in_l, a.rank_capital, b.rank_capital
    );
    let ld = format!(
        r#"{{"@context":"https://schema.org","@type":"WebPage","name":"{}","url":"{}{}","about":["{}","{}"]}}"#,
        jesc(&title),
        BASE,
        path,
        jesc(&a.t.name),
        jesc(&b.t.name)
    );
    let days = |d: Option<u32>| d.map(|x| format!(" · {x} days")).unwrap_or_default();
    let body = format!(
        r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/state/{as}">{an}</a><a href="/state/{bs}">{bn}</a><a href="/from/{bs}/to/{as}">Flip</a></nav>
<p class="muted">Visa corridor</p>
<h1>{af} {an} → {bf} {bn}</h1>
<p>Outbound access for a <strong>{an}</strong> passport into <strong>{bn}</strong>: <strong>{out}</strong>{outd}.</p>
<p>Reverse — <strong>{bn}</strong> into <strong>{an}</strong>: <strong>{inn}</strong>{ind}.</p>
<h2>States</h2>
<table>
<tr><th>From</th><td><a href="/state/{as}">{an}</a> · capital #{ar} · freedom #{afr}</td></tr>
<tr><th>To</th><td><a href="/state/{bs}">{bn}</a> · capital #{br} · hospitality #{bh}</td></tr>
</table>
<p class="muted">Data from the cyberstates visa matrix. Full interactive terminal loads when JavaScript is available.</p>
"##,
        as = a.t.slug,
        bs = b.t.slug,
        an = esc(&a.t.name),
        bn = esc(&b.t.name),
        af = a.t.flag,
        bf = b.t.flag,
        out = out_l,
        inn = in_l,
        outd = days(out_d),
        ind = days(in_d),
        ar = a.rank_capital,
        afr = a.rank_freedom,
        br = b.rank_capital,
        bh = b.rank_hospitality,
    );
    shell_page(&title, &desc, &path, &ld, &body, assets)
}

fn ranking_body(
    heading: &str,
    states: &[State],
    rank_of: impl Fn(&State) -> usize,
    value_of: impl Fn(&State) -> String,
) -> String {
    let mut order: Vec<&State> = states.iter().collect();
    order.sort_by_key(|s| rank_of(s));
    let rows: String = order
        .iter()
        .take(100)
        .map(|s| {
            format!(
                "<tr><td>#{}</td><td><a href=\"/state/{}\">{} {}</a></td><td>{}</td></tr>",
                rank_of(s),
                s.t.slug,
                s.t.flag,
                esc(&s.t.name),
                esc(&value_of(s))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">Tokens</a><a href="/doctrine">Doctrine</a></nav>
<h1>States by {h}</h1>
<p>Top 100 on the sovereignty terminal. Open any row for full fundamentals and visa matrix.</p>
<table><thead><tr><th>Rank</th><th>State</th><th>Value</th></tr></thead><tbody>
{rows}
</tbody></table>
"##,
        h = esc(heading),
        rows = rows
    )
}

fn tokens_index_body(tokens: &[TokenAgg]) -> String {
    let rows: String = tokens
        .iter()
        .take(100)
        .enumerate()
        .map(|(i, t)| {
            format!(
                r#"<tr><td>#{}</td><td><a href="/token/{}">{}</a></td><td>{}</td><td>${:.0}B</td><td>{} states</td></tr>"#,
                i + 1,
                t.code.to_lowercase(),
                esc(&t.code),
                esc(&t.name),
                t.total_cap_b,
                t.states.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/doctrine">Doctrine</a></nav>
<h1>Tokens</h1>
<p>Currency tokens ranked by aggregate capital held across cyberstates.</p>
<table><thead><tr><th>#</th><th>Code</th><th>Name</th><th>Capital</th><th>States</th></tr></thead><tbody>
{rows}
</tbody></table>
"##,
        rows = rows
    )
}

fn doctrine_body() -> String {
    r##"<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">Tokens</a></nav>
<h1>Doctrine</h1>
<p>A state is a territory with a population that establishes its rules. Recognition is opinion; the ground is a ledger of facts.</p>
<p>The first rule is the <strong>token</strong> — what is money here. Capital stock is the market’s running verdict on the rule-set.</p>
<p>Sovereignty is a stack of rule domains, not a binary. States trade on a market for inhabitants and capital — borders and citizenship are microstructure.</p>
<p>Read the full doctrine in the live terminal at <a href="/doctrine">/doctrine</a> (markdown rendered when JavaScript runs), or start from rankings:</p>
<ul>
<li><a href="/by/capital">By capital</a></li>
<li><a href="/by/travel-freedom">By travel freedom</a></li>
<li><a href="/by/hospitality">By hospitality</a></li>
</ul>
"##.into()
}

fn home_body(states: &[State]) -> String {
    let mut order: Vec<&State> = states.iter().filter(|s| is_corridor_eligible(s)).collect();
    order.sort_by_key(|s| s.rank_capital);
    let rows: String = order
        .iter()
        .take(50)
        .map(|s| {
            format!(
                r#"<li><a href="/state/{}">{} {}</a> — ${:.0}B capital · freedom #{}</li>"#,
                s.t.slug,
                s.t.flag,
                esc(&s.t.name),
                s.t.money_supply_b_usd,
                s.rank_freedom
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r##"<nav class="nav"><a href="/tokens">Tokens</a><a href="/by/travel-freedom">Freedom</a><a href="/by/hospitality">Hospitality</a><a href="/doctrine">Doctrine</a></nav>
<h1>Cyberstates — the sovereignty terminal</h1>
<p>Global rankings of states by capital stock, travel freedom and hospitality. Token as the deepest rule. Measured stocks — not GDP press releases.</p>
<h2>Top capital</h2>
<ul>{rows}</ul>
<p class="muted">Full interactive table loads with JavaScript. <a href="/sitemap.xml">Sitemap</a> lists every state, token and visa corridor.</p>
"##,
        rows = rows
    )
}

fn inject_body_into_shell(shell: &str, body_inner: &str) -> String {
    // Replace <body>...</body> with crawlable main + keep scripts in head.
    let main = format!(
        r#"<body>
<main class="seo-crawl" id="seo-static" style="font-family:system-ui,sans-serif;max-width:42rem;margin:1.5rem auto;padding:0 1rem 4rem;color:#e8e8e8;background:#000;line-height:1.55">
{body}
</main>
</body>"#,
        body = body_inner
    );
    if let Some(start) = shell.find("<body") {
        if let Some(end_rel) = shell[start..].find("</body>") {
            let end = start + end_rel + "</body>".len();
            let mut out = String::new();
            out.push_str(&shell[..start]);
            out.push_str(&main);
            out.push_str(&shell[end..]);
            return out;
        }
    }
    shell.to_string()
}

fn landing_pages(_states: &[State], _tokens: &[TokenAgg], _assets: &str) -> Vec<(String, String)> {
    // handled explicitly in main
    Vec::new()
}
