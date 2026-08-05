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
    /// Full tri-kernel-style scores (√ eco×pop reach), same formula as the app.
    freedom: f64,
    openness: f64,
    rank_freedom: usize,
    rank_hospitality: usize,
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
    let tokens = aggregate_tokens(&states);

    // static SEO assets
    let _ = fs::copy("assets/og.png", dist.join("og.png"));
    write_page(&dist.join("404.html"), &page_404(&head_assets));
    write_code_redirects(&dist, &states);
    write_indexnow_key(&dist);

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
            let html = page_corridor(a, b, &states, &head_assets);
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
                |s| {
                    format!(
                        "{:.1} freedom · {} visa-free destinations",
                        s.freedom, s.t.visa_free_destinations
                    )
                },
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
                |s| {
                    format!(
                        "{:.1} hospitality · {} visa-free inbound",
                        s.openness, s.t.visa_free_inbound
                    )
                },
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

    // Enrich root index: crawlable directory + WebSite/Organization JSON-LD
    let root_body = home_body(&states);
    let root_ld = ld_graph(&[
        website_node(),
        org_node(),
        webpage_node(
            "Cyberstates — the sovereignty terminal",
            "/",
            "Global rankings of states by capital stock, travel freedom and hospitality.",
        ),
    ]);
    let root_html = inject_body_into_shell(&shell, &root_body, &root_ld);
    fs::write(dist.join("index.html"), root_html).expect("write root index");

    eprintln!(
        "prerender: {} pages in {:.1}s → {}",
        n_pages,
        t0.elapsed().as_secs_f64(),
        dist.display()
    );
}

fn access_type_weight(t: &str) -> f64 {
    match t {
        "visa-free" => 1.0,
        "visa-on-arrival" => 0.8,
        "eta" | "e-visa" => 0.5,
        "visa-required" => 0.1,
        "no-admission" => 0.0,
        _ => 0.0,
    }
}

fn is_terrestrial(region: &str) -> bool {
    !matches!(region, "Oceans" | "Terra Nullius" | "Solar System")
}

/// Same freedom/openness construction as src/data.rs index_cache.
fn compute_indexes(raw: &[StateToml]) -> HashMap<String, (f64, f64)> {
    let total_cap: f64 = raw
        .iter()
        .filter(|c| is_terrestrial(&c.region) && !AGGREGATES.contains(&c.code.as_str()))
        .map(|c| c.money_supply_b_usd)
        .sum();
    let total_pop: f64 = raw
        .iter()
        .filter(|c| is_terrestrial(&c.region) && !AGGREGATES.contains(&c.code.as_str()))
        .map(|c| c.population as f64)
        .sum();

    let by_name: HashMap<&str, &StateToml> = raw.iter().map(|c| (c.name.as_str(), c)).collect();
    let by_code: HashMap<String, &StateToml> =
        raw.iter().map(|c| (c.code.to_uppercase(), c)).collect();

    let mut acc: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    for holder in raw {
        for e in &holder.visa_access {
            let w = access_type_weight(&e.access_type);
            if let Some(dest) = by_name.get(e.country.as_str()) {
                let out = acc.entry(holder.code.clone()).or_default();
                out.0 += w * dest.money_supply_b_usd;
                out.1 += w * dest.population as f64;
                let inn = acc.entry(dest.code.clone()).or_default();
                inn.2 += w * holder.money_supply_b_usd;
                inn.3 += w * holder.population as f64;
            }
        }
    }
    // touch by_code so unused-warn free if empty matrix rows
    let _ = by_code.len();

    raw.iter()
        .map(|c| {
            let (eco_out, pop_out, eco_in, pop_in) = acc.get(&c.code).copied().unwrap_or_default();
            let eco_out_pct = if total_cap > 0.0 {
                eco_out / total_cap * 100.0
            } else {
                0.0
            };
            let eco_in_pct = if total_cap > 0.0 {
                eco_in / total_cap * 100.0
            } else {
                0.0
            };
            let pop_out_pct = if total_pop > 0.0 {
                pop_out / total_pop * 100.0
            } else {
                0.0
            };
            let pop_in_pct = if total_pop > 0.0 {
                pop_in / total_pop * 100.0
            } else {
                0.0
            };
            (
                c.code.clone(),
                (
                    (eco_out_pct * pop_out_pct).max(0.0).sqrt(),
                    (eco_in_pct * pop_in_pct).max(0.0).sqrt(),
                ),
            )
        })
        .collect()
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
    let indexes = compute_indexes(&raw);
    let n = raw.len();
    let scores: Vec<(f64, f64)> = raw
        .iter()
        .map(|t| indexes.get(&t.code).copied().unwrap_or((0.0, 0.0)))
        .collect();

    let mut idx: Vec<usize> = (0..n).collect();
    let mut rank_capital = vec![0; n];
    let mut rank_pop = vec![0; n];
    let mut rank_area = vec![0; n];
    let mut rank_freedom = vec![0; n];
    let mut rank_hospitality = vec![0; n];

    let assign_raw =
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
    let assign_score =
        |rank: &mut [usize], idx: &mut [usize], scores: &[(f64, f64)], which: usize| {
            idx.sort_by(|&a, &b| {
                let sa = if which == 0 { scores[a].0 } else { scores[a].1 };
                let sb = if which == 0 { scores[b].0 } else { scores[b].1 };
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            for (r, &i) in idx.iter().enumerate() {
                rank[i] = r + 1;
            }
        };

    assign_raw(&mut rank_capital, &mut idx, &raw, |s| s.money_supply_b_usd);
    assign_raw(&mut rank_pop, &mut idx, &raw, |s| s.population as f64);
    assign_raw(&mut rank_area, &mut idx, &raw, |s| s.land_area_km2 as f64);
    assign_score(&mut rank_freedom, &mut idx, &scores, 0);
    assign_score(&mut rank_hospitality, &mut idx, &scores, 1);

    raw.into_iter()
        .enumerate()
        .map(|(i, t)| State {
            t,
            rank_capital: rank_capital[i],
            rank_pop: rank_pop[i],
            rank_area: rank_area[i],
            freedom: scores[i].0,
            openness: scores[i].1,
            rank_freedom: rank_freedom[i],
            rank_hospitality: rank_hospitality[i],
        })
        .collect()
}

fn write_code_redirects(dist: &Path, states: &[State]) {
    let mut lines = vec![
        "# generated by prerender — do not edit".to_string(),
        "# include inside server { } for cyberstates.net".to_string(),
    ];
    for s in states {
        let code = s.t.code.to_lowercase();
        if code != s.t.slug {
            lines.push(format!(
                "location = /state/{code} {{ return 301 /state/{slug}; }}",
                code = code,
                slug = s.t.slug
            ));
            lines.push(format!(
                "location = /state/{code}/ {{ return 301 /state/{slug}; }}",
                code = code,
                slug = s.t.slug
            ));
            lines.push(format!(
                "location = /country/{code} {{ return 301 /state/{slug}; }}",
                code = code,
                slug = s.t.slug
            ));
        }
        lines.push(format!(
            "location = /country/{slug} {{ return 301 /state/{slug}; }}",
            slug = s.t.slug
        ));
        lines.push(format!(
            "location = /country/{slug}/ {{ return 301 /state/{slug}; }}",
            slug = s.t.slug
        ));
    }
    lines.push("location = /methodology { return 301 /doctrine; }".into());
    lines.push("location = /methodology/ { return 301 /doctrine; }".into());
    fs::write(dist.join("nginx-redirects.conf"), lines.join("\n") + "\n").unwrap();
    eprintln!("  redirects:  nginx-redirects.conf");
}

fn write_indexnow_key(dist: &Path) {
    let key = "cyberstates-indexnow-8f3a2c1e9b7d4a60";
    fs::write(dist.join(format!("{key}.txt")), key).unwrap();
    fs::write(dist.join("indexnow-key.txt"), format!("{key}\n")).unwrap();
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

/// JSON-LD @graph wrapper.
fn ld_graph(nodes: &[String]) -> String {
    format!(
        r#"{{"@context":"https://schema.org","@graph":[{}]}}"#,
        nodes.join(",")
    )
}

/// BreadcrumbList node. `trail` is (name, absolute path) including current page.
fn ld_breadcrumb(trail: &[(&str, &str)]) -> String {
    let items: Vec<String> = trail
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            format!(
                r#"{{"@type":"ListItem","position":{},"name":"{}","item":"{}{}"}}"#,
                i + 1,
                jesc(name),
                BASE,
                path
            )
        })
        .collect();
    format!(
        r#"{{"@type":"BreadcrumbList","itemListElement":[{}]}}"#,
        items.join(",")
    )
}

/// Visible breadcrumb nav for internal linking + UX.
fn html_breadcrumb(trail: &[(&str, &str)]) -> String {
    let parts: Vec<String> = trail
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            if i + 1 == trail.len() {
                format!(r#"<span aria-current="page">{}</span>"#, esc(name))
            } else {
                format!(r#"<a href="{}">{}</a>"#, path, esc(name))
            }
        })
        .collect();
    format!(
        r#"<nav class="crumbs" aria-label="Breadcrumb">{}</nav>"#,
        parts.join(" <span class=\"sep\">/</span> ")
    )
}

fn website_node() -> String {
    format!(
        r#"{{"@type":"WebSite","@id":"{base}/#website","url":"{base}/","name":"Cyberstates","description":"Sovereignty terminal — states ranked by capital, travel freedom and hospitality.","publisher":{{"@id":"{base}/#org"}},"inLanguage":"en"}}"#,
        base = BASE
    )
}

fn org_node() -> String {
    format!(
        r#"{{"@type":"Organization","@id":"{base}/#org","name":"Cyberstates","url":"{base}/","logo":"{base}/og.png","sameAs":["https://x.com/cyberiacap"]}}"#,
        base = BASE
    )
}

fn webpage_node(name: &str, path: &str, desc: &str) -> String {
    format!(
        r#"{{"@type":"WebPage","@id":"{base}{path}#webpage","url":"{base}{path}","name":"{name}","description":"{desc}","isPartOf":{{"@id":"{base}/#website"}},"inLanguage":"en"}}"#,
        base = BASE,
        path = path,
        name = jesc(name),
        desc = jesc(desc)
    )
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
<meta property="og:image" content="{base}/og.png" />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{desc}" />
<meta name="twitter:image" content="{base}/og.png" />
<meta name="theme-color" content="#000000" />
<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
{assets}
<script type="application/ld+json">{ld}</script>
</head>
<body>
<!-- Crawlable copy for bots/no-JS. Hidden from view; SPA strips #seo-static on boot. -->
<div id="seo-static" hidden>
{body}
</div>
<noscript>
<main style="font-family:system-ui,sans-serif;max-width:42rem;margin:1.5rem auto;padding:0 1rem 4rem;color:#e8e8e8;background:#000;line-height:1.55">
{body}
</main>
</noscript>
</body>
</html>
"##,
        title = title_e,
        desc = desc_e,
        canon = canon_e,
        base = BASE,
        assets = head_assets,
        ld = json_ld,
        body = body_inner,
    )
}

fn page_404(assets: &str) -> String {
    // Built without shell_page index robots — inject noindex via dedicated shell.
    let body = r#"<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">Tokens</a><a href="/doctrine">Doctrine</a></nav>
<h1>404 — not found</h1>
<p>No state, token, or visa corridor at this URL. Start from the <a href="/">terminal</a>, <a href="/tokens">tokens</a>, or the <a href="/sitemap.xml">sitemap</a>.</p>"#;
    let html = shell_page(
        "404 — not found | Cyberstates",
        "This path is not a listed cyberstate, token, or corridor.",
        "/404",
        r#"{"@context":"https://schema.org","@type":"WebPage","name":"404"}"#,
        body,
        assets,
    );
    html.replace(
        "content=\"index, follow, max-image-preview:large, max-snippet:-1\"",
        "content=\"noindex, follow\"",
    )
}

fn page_simple(title: &str, desc: &str, path: &str, body: &str, assets: &str) -> String {
    let trail: Vec<(&str, &str)> = if path == "/" {
        vec![("Cyberstates", "/")]
    } else {
        let label = title.split(" — ").next().unwrap_or(title);
        // short label without rank noise for crumbs
        let short = label.split(" | ").next().unwrap_or(label);
        vec![("Cyberstates", "/"), (short, path)]
    };
    // For trail with owned strings on ranking pages:
    let crumb_name = title
        .split(" | ")
        .next()
        .unwrap_or(title)
        .split(" — ")
        .next()
        .unwrap_or(title);
    let trail_owned = [("Cyberstates", "/"), (crumb_name, path)];
    let trail_use: &[(&str, &str)] = if path == "/" { &trail } else { &trail_owned };
    let ld = ld_graph(&[
        website_node(),
        org_node(),
        webpage_node(title, path, desc),
        ld_breadcrumb(trail_use),
    ]);
    let body = format!("{}{}", html_breadcrumb(trail_use), body);
    shell_page(title, desc, path, &ld, &body, assets)
}

fn page_state(s: &State, all: &[State], assets: &str) -> String {
    let path = format!("/state/{}", s.t.slug);
    let title = format!(
        "{} — capital #{}, freedom {:.1} (#{}) | Cyberstates",
        s.t.name, s.rank_capital, s.freedom, s.rank_freedom
    );
    let desc = format!(
        "{} ({}): money stock ${:.0}B (rank #{}), freedom {:.1} (#{}, √eco×pop reach), hospitality {:.1} (#{}). Token {}. Population {}.",
        s.t.name,
        s.t.code,
        s.t.money_supply_b_usd,
        s.rank_capital,
        s.freedom,
        s.rank_freedom,
        s.openness,
        s.rank_hospitality,
        s.t.currency_code,
        s.t.population
    );
    let place = format!(
        r#"{{"@type":"Place","@id":"{base}{path}#place","name":"{name}","identifier":"{code}","url":"{base}{path}","image":"{base}/og.png","containedInPlace":"{region}","additionalProperty":[{{"@type":"PropertyValue","name":"capital_b_usd","value":{cap}}},{{"@type":"PropertyValue","name":"population","value":{pop}}},{{"@type":"PropertyValue","name":"land_area_km2","value":{area}}},{{"@type":"PropertyValue","name":"currency","value":"{cur}"}},{{"@type":"PropertyValue","name":"travel_freedom","value":{fs}}},{{"@type":"PropertyValue","name":"hospitality","value":{os}}}]}}"#,
        base = BASE,
        path = path,
        name = jesc(&s.t.name),
        code = jesc(&s.t.code),
        region = jesc(&s.t.region),
        cap = s.t.money_supply_b_usd,
        pop = s.t.population,
        area = s.t.land_area_km2,
        cur = jesc(&s.t.currency_code),
        fs = s.freedom,
        os = s.openness
    );
    let trail = [
        ("Cyberstates", "/"),
        ("States", "/by/capital"),
        (s.t.name.as_str(), path.as_str()),
    ];
    let ld = ld_graph(&[
        website_node(),
        org_node(),
        webpage_node(&title, &path, &desc),
        ld_breadcrumb(&trail),
        place,
    ]);

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

    let trail_html = html_breadcrumb(&[
        ("Cyberstates", "/"),
        ("States", "/by/capital"),
        (s.t.name.as_str(), path.as_str()),
    ]);
    let body = format!(
        r##"{crumbs}
<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">Tokens</a><a href="/by/travel-freedom">Freedom</a><a href="/by/capital">Capital</a><a href="/doctrine">Doctrine</a></nav>
<p class="muted">{flag} {region} · {code}</p>
<h1>{name}</h1>
<p>{name} is listed as a cyberstate — territory, population and rules measured on the sovereignty terminal. Token <a href="/token/{tok}">{tok}</a> ({tokn}).</p>
<h2>Fundamentals</h2>
<table>
<tr><th>Capital (money stock)</th><td>${cap:.1}B · rank #{rc} of {n}</td></tr>
<tr><th>Population</th><td>{pop} · rank #{rp}</td></tr>
<tr><th>Territory</th><td>{area} km² · rank #{ra}</td></tr>
<tr><th>Token price</th><td>{price}</td></tr>
<tr><th>Travel freedom</th><td>{fs:.1} (√ eco×pop reach) · rank #{rf} · {vf} visa-free destinations</td></tr>
<tr><th>Hospitality</th><td>{os:.1} · rank #{rh} · {vi} visa-free inbound</td></tr>
</table>
<h2>Related rankings</h2>
<ul>
<li><a href="/by/capital">All states by capital</a></li>
<li><a href="/by/travel-freedom">All states by travel freedom</a></li>
<li><a href="/by/hospitality">All states by hospitality</a></li>
<li><a href="/token/{tok}">All states on {tok}</a></li>
</ul>
<h2>Visa-free destinations</h2>
<ul>{free}</ul>
{more_free}
<h2>Corridors from {name}</h2>
<ul>{corr}</ul>
<p class="muted">Every ordered pair is listed under /from/{slug}/to/… · <a href="/sitemap.xml">Sitemap</a></p>
"##,
        crumbs = trail_html,
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
        fs = s.freedom,
        os = s.openness,
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
    let fin = format!(
        r#"{{"@type":"ExchangeRateSpecification","@id":"{base}{path}#token","currency":"{code}","url":"{base}{path}","name":"{name}","description":"{desc}","currentExchangeRate":{{"@type":"UnitPriceSpecification","price":{price},"priceCurrency":"USD"}}}}"#,
        base = BASE,
        path = path,
        code = jesc(&t.code),
        name = jesc(&t.name),
        desc = jesc(&desc),
        price = if t.price_usd > 0.0 {
            format!("{:.8}", t.price_usd)
        } else {
            "0".into()
        },
    );
    // Note: outer object closes with }} — one for currentExchangeRate, one for ExchangeRateSpecification
    let trail = [
        ("Cyberstates", "/"),
        ("Tokens", "/tokens"),
        (t.code.as_str(), path.as_str()),
    ];
    let ld = ld_graph(&[
        website_node(),
        org_node(),
        webpage_node(&title, &path, &desc),
        ld_breadcrumb(&trail),
        fin,
    ]);
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
        r##"{crumbs}
<nav class="nav"><a href="/">Cyberstates</a><a href="/tokens">All tokens</a><a href="/by/capital">Capital</a><a href="/doctrine">Doctrine</a></nav>
<h1>{code} — {name}</h1>
<p>Token of record for {n} cyberstates. Aggregate capital ${cap:.1}B. Price {price}.</p>
<h2>States on {code}</h2>
<ul>{rows}</ul>
<p class="muted"><a href="/tokens">All tokens</a> · <a href="/sitemap.xml">Sitemap</a></p>
"##,
        crumbs = html_breadcrumb(&trail),
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

fn page_corridor(a: &State, b: &State, all: &[State], assets: &str) -> String {
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
        "Can {} passport holders enter {}? {}. Reverse: {}. Freedom {:.1} → hospitality {:.1}. Capital #{} → #{}.",
        a.t.name, b.t.name, out_l, in_l, a.freedom, b.openness, a.rank_capital, b.rank_capital
    );
    let a_path = format!("/state/{}", a.t.slug);
    let b_path = format!("/state/{}", b.t.slug);
    let corr_label = format!("{} → {}", a.t.name, b.t.name);
    let trail = [
        ("Cyberstates", "/"),
        (a.t.name.as_str(), a_path.as_str()),
        (corr_label.as_str(), path.as_str()),
    ];
    let about = format!(
        r#"{{"@type":"WebPage","@id":"{base}{path}#webpage","url":"{base}{path}","name":"{title}","description":"{desc}","about":[{{"@type":"Place","name":"{an}","url":"{base}{ap}"}},{{"@type":"Place","name":"{bn}","url":"{base}{bp}"}}],"isPartOf":{{"@id":"{base}/#website"}},"inLanguage":"en"}}"#,
        base = BASE,
        path = path,
        title = jesc(&title),
        desc = jesc(&desc),
        an = jesc(&a.t.name),
        bn = jesc(&b.t.name),
        ap = a_path,
        bp = b_path,
    );
    let ld = ld_graph(&[website_node(), org_node(), about, ld_breadcrumb(&trail)]);
    let days = |d: Option<u32>| d.map(|x| format!(" · {x} days")).unwrap_or_default();

    // Related: reverse, same-region peers, same-token peers
    let mut related = Vec::new();
    related.push(format!(
        r#"<li><a href="/from/{}/to/{}">Reverse: {} → {}</a></li>"#,
        b.t.slug,
        a.t.slug,
        esc(&b.t.name),
        esc(&a.t.name)
    ));
    let mut region_peers: Vec<&State> = all
        .iter()
        .filter(|s| {
            is_corridor_eligible(s)
                && s.t.region == b.t.region
                && s.t.slug != b.t.slug
                && s.t.slug != a.t.slug
        })
        .collect();
    region_peers.sort_by_key(|s| s.rank_capital);
    for p in region_peers.iter().take(6) {
        related.push(format!(
            r#"<li><a href="/from/{}/to/{}">{} → {}</a> (same region as destination)</li>"#,
            a.t.slug,
            p.t.slug,
            esc(&a.t.name),
            esc(&p.t.name)
        ));
    }
    let mut token_peers: Vec<&State> = all
        .iter()
        .filter(|s| {
            is_corridor_eligible(s)
                && s.t.currency_code == a.t.currency_code
                && s.t.slug != a.t.slug
        })
        .collect();
    token_peers.sort_by_key(|s| s.rank_capital);
    for p in token_peers.iter().take(4) {
        related.push(format!(
            r#"<li><a href="/from/{}/to/{}">{} → {}</a> (same token {})</li>"#,
            p.t.slug,
            b.t.slug,
            esc(&p.t.name),
            esc(&b.t.name),
            esc(&a.t.currency_code)
        ));
    }

    let body = format!(
        r##"{crumbs}
<nav class="nav"><a href="/">Cyberstates</a><a href="/state/{as}">{an}</a><a href="/state/{bs}">{bn}</a><a href="/from/{bs}/to/{as}">Flip</a><a href="/token/{tok}">Token</a></nav>
<p class="muted">Visa corridor</p>
<h1>{af} {an} → {bf} {bn}</h1>
<p>Outbound access for a <strong>{an}</strong> passport into <strong>{bn}</strong>: <strong>{out}</strong>{outd}.</p>
<p>Reverse — <strong>{bn}</strong> into <strong>{an}</strong>: <strong>{inn}</strong>{ind}.</p>
<h2>States</h2>
<table>
<tr><th>From</th><td><a href="/state/{as}">{an}</a> · capital #{ar} · freedom {afs:.1} (#{afr})</td></tr>
<tr><th>To</th><td><a href="/state/{bs}">{bn}</a> · capital #{br} · hospitality {bos:.1} (#{bh})</td></tr>
</table>
<h2>Related corridors</h2>
<ul>{related}</ul>
<p class="muted">Data from the cyberstates visa matrix · <a href="/sitemap.xml">Sitemap</a></p>
"##,
        crumbs = html_breadcrumb(&trail),
        as = a.t.slug,
        bs = b.t.slug,
        an = esc(&a.t.name),
        bn = esc(&b.t.name),
        af = a.t.flag,
        bf = b.t.flag,
        tok = a.t.currency_code.to_lowercase(),
        out = out_l,
        inn = in_l,
        outd = days(out_d),
        ind = days(in_d),
        ar = a.rank_capital,
        afr = a.rank_freedom,
        afs = a.freedom,
        br = b.rank_capital,
        bh = b.rank_hospitality,
        bos = b.openness,
        related = related.join("\n"),
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
                r#"<li><a href="/state/{}">{} {}</a> — ${:.0}B capital · freedom {:.1} (#{})</li>"#,
                s.t.slug,
                s.t.flag,
                esc(&s.t.name),
                s.t.money_supply_b_usd,
                s.freedom,
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

fn inject_body_into_shell(shell: &str, body_inner: &str, json_ld: &str) -> String {
    // Hidden crawlable directory + noscript. Never paint under the SPA.
    // Inject JSON-LD before </head> for the home shell.
    let shell = if let Some(i) = shell.rfind("</head>") {
        format!(
            "{}<script type=\"application/ld+json\">{}</script>\n{}",
            &shell[..i],
            json_ld,
            &shell[i..]
        )
    } else {
        shell.to_string()
    };
    let main = format!(
        r#"<body>
<div id="seo-static" hidden>
{body}
</div>
<noscript>
<main style="font-family:system-ui,sans-serif;max-width:42rem;margin:1.5rem auto;padding:0 1rem 4rem;color:#e8e8e8;background:#000;line-height:1.55">
{body}
</main>
</noscript>
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
