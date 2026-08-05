//! Scrapes visa data directly from passportindex.org via Chrome DevTools Protocol.
//!
//! USAGE:
//!   1. Start Chrome: /Applications/Google\ Chrome.app/Contents/MacOS/Google\ Chrome \
//!        --remote-debugging-port=9222 --user-data-dir=/tmp/chrome-scraper
//!   2. Open https://www.passportindex.org/byRank.php in that Chrome, wait for Cloudflare.
//!   3. Run: cargo run --bin scrape-passportindex --features scraper
//!
//! Options:
//!   --country=DE     Scrape only one country
//!   --debug          Print raw extracted data
//!   --dump           Dump page DOM and exit
//!   --port=9222      Chrome debug port (default: 9222)

use headless_chrome::Browser;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Passportindex destination name → our TOML name
fn pi_to_toml_name() -> HashMap<String, String> {
    let pairs = [
        ("Czech Republic", "Czechia"),
        ("Congo (Dem. Rep.)", "Democratic Republic of the Congo"),
        ("Cote d'Ivoire (Ivory Coast)", "Cote d'Ivoire"),
        ("Russian Federation", "Russia"),
        ("Türkiye", "Turkey"),
        ("Viet Nam", "Vietnam"),
        ("Palestinian Territories", "Palestine"),
        (
            "St. Vincent and the Grenadines",
            "Saint Vincent and the Grenadines",
        ),
        ("Hong Kong", "Hong Kong"),
        ("Macao", "Macao"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn wait_for_content(tab: &Arc<headless_chrome::Tab>, timeout_secs: u64) -> bool {
    for _ in 0..timeout_secs * 4 {
        std::thread::sleep(Duration::from_millis(250));
        if let Ok(result) = tab.evaluate("document.title", false) {
            if let Some(title) = result.value.as_ref().and_then(|v| v.as_str()) {
                if !title.contains("moment")
                    && !title.contains("Checking")
                    && !title.contains("Verifying")
                    && !title.is_empty()
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Step 1: Parse byRank.php to get all passport slugs.
/// Returns: Vec<(display_name, slug)> — e.g. ("Germany", "germany")
fn scrape_index(tab: &Arc<headless_chrome::Tab>) -> Vec<(String, String)> {
    let js = r#"(() => {
        const results = [];

        // Strategy 1: Find all links containing /passport/
        document.querySelectorAll('a[href*="/passport/"]').forEach(a => {
            const href = a.getAttribute('href') || '';
            const match = href.match(/\/passport\/([^\/\?#]+)/);
            if (!match) return;
            const slug = match[1];
            const name = a.textContent.trim().replace(/^\d+[\.\s]*/, '').trim();
            if (name && slug && name.length > 1 && name.length < 60 && slug !== 'index') {
                results.push({ name, slug });
            }
        });

        // Strategy 2: Find links with onclick or data attributes pointing to passports
        if (results.length === 0) {
            document.querySelectorAll('a, [onclick], [data-href]').forEach(el => {
                const href = el.getAttribute('href') || el.getAttribute('onclick') || el.getAttribute('data-href') || '';
                const match = href.match(/passport\/([a-z][a-z0-9-]+)/i);
                if (!match) return;
                const slug = match[1].toLowerCase();
                const name = el.textContent.trim().replace(/^\d+[\.\s]*/, '').trim();
                if (name && slug && name.length > 1 && name.length < 60) {
                    results.push({ name, slug });
                }
            });
        }

        // Strategy 3: Parse the visible text for country names + extract slugs from all links on page
        if (results.length === 0) {
            // Get all unique hrefs
            const allLinks = {};
            document.querySelectorAll('a').forEach(a => {
                const href = a.getAttribute('href') || '';
                const text = a.textContent.trim();
                if (text && href) allLinks[text] = href;
            });

            // Look for country-like text entries in the page body
            const bodyText = document.body?.innerText || '';
            const lines = bodyText.split('\n').map(l => l.trim()).filter(l => l.length > 1 && l.length < 60);

            // Find the section with ranked passports
            let inRanking = false;
            for (const line of lines) {
                if (line.match(/^1[\.\s]/) || line.match(/Rank.*1/i)) inRanking = true;
                if (!inRanking) continue;

                // Lines like "1. GERMANY" or just country names in CAPS
                const cleaned = line.replace(/^\d+[\.\s]+/, '').trim();
                if (!cleaned || cleaned.length < 2 || cleaned.length > 50) continue;
                // Skip junk
                if (cleaned.match(/AVERAGE|MEDIAN|POPULATION|BECOME|EMPOWERED|NEWSLETTER|VISA CHECKER|EXPLORE|RANK|COMPARE|IMPROVE|DISCOVER/i)) continue;

                // Title case: "UNITED STATES" -> "United States"
                const titleCase = cleaned.toLowerCase().replace(/(?:^|\s|[-'])\w/g, c => c.toUpperCase());
                const slug = cleaned.toLowerCase().replace(/[\s]+/g, '-').replace(/[^a-z0-9-]/g, '');
                if (slug.length > 2) {
                    results.push({ name: titleCase, slug });
                }
            }
        }

        // Deduplicate by slug
        const seen = new Set();
        const unique = [];
        for (const r of results) {
            if (!seen.has(r.slug)) {
                seen.add(r.slug);
                unique.push(r);
            }
        }
        return JSON.stringify(unique);
    })()"#;

    let raw = tab
        .evaluate(js, false)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "[]".to_string());

    #[derive(Deserialize)]
    struct Entry {
        name: String,
        slug: String,
    }

    serde_json::from_str::<Vec<Entry>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(|e| (e.name, e.slug))
        .collect()
}

/// Step 2: Extract visa data from a passport page
fn extract_visa_data(tab: &Arc<headless_chrome::Tab>) -> String {
    let js = r#"(() => {
        const results = [];
        const text = document.body?.innerText || '';
        const lines = text.split('\n');

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed === 'Apply Now' || trimmed.startsWith('BECOME')
                || trimmed.startsWith('EMPOWERED') || trimmed.startsWith('NEWSLETTER')
                || trimmed.startsWith('©') || trimmed === 'Visa Checker'
                || trimmed.startsWith('Find a country') || trimmed === 'Country'
                || trimmed === 'Visa requirements') continue;

            const parts = trimmed.split('\t');
            if (parts.length < 2) continue;

            const country = parts[0].replace(/^[^\w]+/, '').trim();
            if (country.length < 2 || country.length > 60) continue;

            const visaPart = parts[1].trim().toUpperCase();
            let type_ = '';
            let days = null;

            if (visaPart.includes('VISA-FREE') || visaPart.includes('VISA FREE')) {
                type_ = 'visa-free';
            } else if (visaPart.includes('VISA ON ARRIVAL') || visaPart.includes('VOA')) {
                type_ = 'visa-on-arrival';
            } else if (visaPart.includes('EVISITOR')) {
                type_ = 'eta';
            } else if (visaPart.includes('EVISA') || visaPart.includes('E-VISA')) {
                type_ = 'e-visa';
            } else if (visaPart.startsWith('ETA')) {
                type_ = 'eta';
            } else if (visaPart.includes('PRE-ENROLLMENT') || visaPart.includes('PRE ENROLLMENT')) {
                type_ = 'e-visa';
            } else if (visaPart.includes('TOURIST REGISTRATION')) {
                type_ = 'visa-free';
            } else if (visaPart.includes('TOURIST CARD')) {
                type_ = 'visa-free';
            } else if (visaPart.includes('E-TICKET')) {
                type_ = 'visa-free';
            } else if (visaPart.includes('VISA REQUIRED') || visaPart.includes('VISA REQ')) {
                type_ = 'visa-required';
            } else if (visaPart.includes('NO ADMISSION') || visaPart.includes('NO ADM')) {
                type_ = 'no-admission';
            }

            const dm = visaPart.match(/(\d+)/);
            if (dm) {
                const d = parseInt(dm[1]);
                if (d > 0 && d <= 360) days = d;
            }

            if (country && type_) {
                results.push({ country, type: type_, days });
            }
        }

        if (results.length > 0) return JSON.stringify(results);
        return JSON.stringify({ error: 'no_data', lineCount: lines.length });
    })()"#;
    tab.evaluate(js, false)
        .ok()
        .and_then(|r| r.value)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "null".to_string())
}

/// Match a passportindex name/slug to our TOML file.
/// Returns (toml_path, toml_country_name) or None.
fn match_to_toml(
    pi_name: &str,
    _slug: &str,
    toml_countries: &[(String, String, String)], // (code, name, filepath)
) -> Option<(String, String)> {
    let pi_lower = pi_name.to_lowercase();

    // Direct match
    for (_, name, path) in toml_countries {
        if name.to_lowercase() == pi_lower {
            return Some((path.clone(), name.clone()));
        }
    }

    // Known aliases
    let aliases: &[(&str, &str)] = &[
        ("russian federation", "russia"),
        ("türkiye", "turkey"),
        ("viet nam", "vietnam"),
        ("czech republic", "czechia"),
        ("palestinian territories", "palestine"),
        ("congo (dem. rep.)", "democratic republic of the congo"),
        ("cote d'ivoire", "cote d'ivoire"),
        (
            "st. vincent and the grenadines",
            "saint vincent and the grenadines",
        ),
    ];

    for (from, to) in aliases {
        if pi_lower == *from {
            for (_, name, path) in toml_countries {
                if name.to_lowercase() == *to {
                    return Some((path.clone(), name.clone()));
                }
            }
        }
    }

    // Fuzzy: check if one contains the other
    for (_, name, path) in toml_countries {
        let toml_lower = name.to_lowercase();
        if toml_lower.contains(&pi_lower) || pi_lower.contains(&toml_lower) {
            return Some((path.clone(), name.clone()));
        }
    }

    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let single_country = args
        .iter()
        .find(|a| a.starts_with("--country="))
        .map(|a| a.strip_prefix("--country=").unwrap().to_uppercase());
    let debug = args.iter().any(|a| a == "--debug");
    let dump_only = args.iter().any(|a| a == "--dump");
    let port: u16 = args
        .iter()
        .find(|a| a.starts_with("--port="))
        .and_then(|a| a.strip_prefix("--port="))
        .and_then(|p| p.parse().ok())
        .unwrap_or(9222);

    eprintln!("=== Passport Index Scraper ===");
    eprintln!("Connecting to Chrome on port {}...", port);

    let version_url = format!("http://127.0.0.1:{}/json/version", port);
    let ws_url = std::process::Command::new("curl")
        .args(["-s", &version_url])
        .output()
        .ok()
        .and_then(|o| {
            let body = String::from_utf8_lossy(&o.stdout).to_string();
            serde_json::from_str::<serde_json::Value>(&body).ok()
        })
        .and_then(|v| v["webSocketDebuggerUrl"].as_str().map(|s| s.to_string()));

    let ws_url = match ws_url {
        Some(url) => {
            eprintln!("Connected: {}", url);
            url
        }
        None => {
            eprintln!("ERROR: Cannot connect to Chrome on port {}.", port);
            eprintln!("Start Chrome with --remote-debugging-port={}", port);
            std::process::exit(1);
        }
    };

    let browser = Browser::connect(ws_url).expect("WebSocket connection failed");
    let tabs = browser.get_tabs().lock().unwrap().clone();
    let tab = tabs
        .first()
        .cloned()
        .unwrap_or_else(|| browser.new_tab().unwrap());

    // Step 1: Navigate to byRank.php to discover all passport slugs
    eprintln!("\n1. Scraping index at byRank.php...");
    tab.navigate_to("https://www.passportindex.org/byRank.php")
        .unwrap();
    if !wait_for_content(&tab, 30) {
        eprintln!("Cloudflare didn't clear. Open byRank.php in Chrome first.");
        return;
    }
    std::thread::sleep(Duration::from_secs(2));

    let index = scrape_index(&tab);
    eprintln!("   Found {} passports on index page", index.len());

    if index.is_empty() {
        // Dump page to diagnose
        let title = tab
            .evaluate("document.title", false)
            .ok()
            .and_then(|r| r.value)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let sample = tab
            .evaluate("document.body?.innerText?.substring(0, 1000) || ''", false)
            .ok()
            .and_then(|r| r.value)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        eprintln!("   Page title: {}", title);
        eprintln!("   Body sample: {}", &sample[..sample.len().min(500)]);

        if title.contains("moment") || title.contains("Checking") {
            eprintln!("\n   Cloudflare is still active.");
            eprintln!("   Open https://www.passportindex.org/byRank.php in Chrome manually,");
            eprintln!("   wait for it to load, then re-run this tool.");
        } else {
            eprintln!("\n   Page loaded but no passport links found.");
            eprintln!("   The site structure may have changed.");
            // Try alternate selector
            let alt = tab
                .evaluate(
                    r#"
                JSON.stringify(Array.from(document.querySelectorAll('a')).filter(a =>
                    (a.href||'').includes('passport')).slice(0,5).map(a =>
                    ({href: a.href, text: a.textContent.trim().substring(0,50)})))
            "#,
                    false,
                )
                .ok()
                .and_then(|r| r.value)
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            eprintln!("   Sample passport links: {}", alt);

            // Dump ALL links for analysis
            let all_links = tab
                .evaluate(
                    r#"
                JSON.stringify(Array.from(new Set(
                    Array.from(document.querySelectorAll('a'))
                        .map(a => a.href)
                        .filter(h => h && !h.startsWith('javascript'))
                )).slice(0, 50))
            "#,
                    false,
                )
                .ok()
                .and_then(|r| r.value)
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            eprintln!("   All links: {}", all_links);
        }
        return;
    }

    if dump_only {
        for (name, slug) in &index {
            eprintln!("  {} -> /passport/{}/", name, slug);
        }
        return;
    }

    // Step 2: Load our TOML files
    let states_dir = Path::new("states");
    let mut toml_countries: Vec<(String, String, String)> = Vec::new();
    let mut dir_entries: Vec<_> = fs::read_dir(states_dir)
        .expect("Cannot read states/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
        .collect();
    dir_entries.sort_by_key(|e| e.file_name());

    for entry in &dir_entries {
        let content = fs::read_to_string(entry.path()).unwrap();
        let name = content
            .lines()
            .find(|l| l.starts_with("name = "))
            .and_then(|l| l.strip_prefix("name = \""))
            .and_then(|l| l.strip_suffix('"'))
            .unwrap_or("")
            .to_string();
        let code = content
            .lines()
            .find(|l| l.starts_with("code = "))
            .and_then(|l| l.strip_prefix("code = \""))
            .and_then(|l| l.strip_suffix('"'))
            .unwrap_or("")
            .to_string();
        if !name.is_empty() && !code.is_empty() {
            toml_countries.push((code, name, entry.path().to_string_lossy().to_string()));
        }
    }

    // Step 3: Match index entries to TOML files, build work list
    let name_map = pi_to_toml_name();
    let mut work: Vec<(String, String, String)> = Vec::new(); // (slug, pi_name, toml_filepath)
    let mut unmatched: Vec<String> = Vec::new();

    for (pi_name, slug) in &index {
        if let Some(ref target) = single_country {
            // Check if this passport matches the target code
            let matched = toml_countries.iter().any(|(code, _, _)| code == target);
            if matched {
                let m = match_to_toml(pi_name, slug, &toml_countries);
                if let Some((path, _)) = m {
                    let target_path = toml_countries
                        .iter()
                        .find(|(c, _, _)| c == target)
                        .map(|(_, _, p)| p.clone());
                    if Some(&path) == target_path.as_ref() {
                        work.push((slug.clone(), pi_name.clone(), path));
                    }
                    continue;
                }
            }
            // Also try matching by name
            if let Some((path, _)) = match_to_toml(pi_name, slug, &toml_countries) {
                let is_target = toml_countries
                    .iter()
                    .any(|(c, _, p)| c == target && p == &path);
                if is_target {
                    work.push((slug.clone(), pi_name.clone(), path));
                }
            }
            continue;
        }

        match match_to_toml(pi_name, slug, &toml_countries) {
            Some((path, _)) => work.push((slug.clone(), pi_name.clone(), path)),
            None => unmatched.push(format!("{} ({})", pi_name, slug)),
        }
    }

    eprintln!("   Matched {} passports to TOML files", work.len());
    if !unmatched.is_empty() {
        eprintln!("   Unmatched ({}):", unmatched.len());
        for u in &unmatched {
            eprintln!("     - {}", u);
        }
    }

    eprintln!("\n2. Scraping {} passport pages...\n", work.len());

    let mut updated = 0;
    let mut failed: Vec<String> = Vec::new();

    for (idx, (slug, pi_name, filepath)) in work.iter().enumerate() {
        let url = format!("https://www.passportindex.org/passport/{}/", slug);
        eprint!("  [{}/{}] {}...", idx + 1, work.len(), pi_name);

        if let Err(e) = tab.navigate_to(&url) {
            eprintln!(" nav error: {}", e);
            failed.push(pi_name.clone());
            continue;
        }

        if !wait_for_content(&tab, 20) {
            eprintln!(" timeout");
            failed.push(pi_name.clone());
            continue;
        }
        std::thread::sleep(Duration::from_millis(1500));

        let raw = extract_visa_data(&tab);
        if debug {
            eprintln!("\n    raw: {}", &raw[..raw.len().min(300)]);
        }

        match serde_json::from_str::<Vec<RawVisaEntry>>(&raw) {
            Ok(entries) if !entries.is_empty() => {
                let content = fs::read_to_string(filepath).unwrap();
                let base: String = content
                    .lines()
                    .take_while(|l| !l.starts_with("[[visa_access]]"))
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut visa_section = String::from("\n");
                for ve in &entries {
                    let dest = name_map
                        .get(&ve.country)
                        .cloned()
                        .unwrap_or_else(|| ve.country.clone());
                    visa_section.push_str("[[visa_access]]\n");
                    visa_section
                        .push_str(&format!("country = \"{}\"\n", escape_toml_string(&dest)));
                    visa_section.push_str(&format!("type = \"{}\"\n", ve.access_type));
                    if let Some(days) = ve.days {
                        visa_section.push_str(&format!("days = {}\n", days));
                    }
                    visa_section.push('\n');
                }

                fs::write(filepath, format!("{}\n{}", base.trim_end(), visa_section)).unwrap();
                updated += 1;
                eprintln!(" {} entries", entries.len());
            }
            _ => {
                eprintln!(" no data");
                failed.push(pi_name.clone());
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    eprintln!("\n=== Done ===");
    eprintln!("Updated: {}", updated);
    if !failed.is_empty() {
        eprintln!("Failed ({}):", failed.len());
        for f in &failed {
            eprintln!("  - {}", f);
        }
    }
}

#[derive(Deserialize, Debug)]
struct RawVisaEntry {
    country: String,
    #[serde(rename = "type")]
    access_type: String,
    days: Option<u32>,
}
