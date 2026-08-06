//! Refreshes token_price_usd and money_supply_b_usd in states/*.toml, and
//! the numeraire constants in src/numeraires.rs. Rust port of
//! update_market_data.py — same sources, same guards, now in the same
//! toolchain as fetch-visa-data and scrape-passportindex.
//!
//! Sources:
//! - FX rates:    v6.exchangerate-api.com (needs EXCHANGERATE_API_KEY —
//!                the free open.er-api.com mirror has no key and a lower
//!                rate limit; the paid v6 endpoint is what the env var
//!                gates)
//! - Broad money: World Bank FM.LBL.BMNY.CN (current LCU, most recent
//!                non-empty year, accepted only if >= MIN_YEAR)
//! - US M2:       FRED M2SL via keyless fredgraph.csv (World Bank stopped
//!                publishing US broad money)
//! - Numeraires:  CoinGecko (BTC, ETH, PAXG=gold)
//!
//! Per state:
//! - price: fresh FX where the currency code exists; otherwise the old
//!   price stays.
//! - supply: World Bank LCU x fresh price where available; otherwise the
//!   old USD cap is revalued by the price change (token supply
//!   preserved), UNLESS the FX moved more than 3x — that reads as a
//!   redenomination or a broken quote, so the cap is kept as-is.
//!
//! Run:
//!   EXCHANGERATE_API_KEY=... cargo run --release --bin update-market-data --features tools
//!
//! The key is never read from a file in this repo — env var only, so it
//! can't end up in a public commit by accident.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const MIN_YEAR: i32 = 2018;
const WB_URL: &str = "https://api.worldbank.org/v2/country/all/indicator/FM.LBL.BMNY.CN?format=json&mrnev=1&per_page=400";
const FRED_URL: &str = "https://fred.stlouisfed.org/graph/fredgraph.csv?id=M2SL";
const CG_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,pax-gold&vs_currencies=usd";

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent("cyberstates-market-updater/1.0")
        .build()
        .expect("failed to build HTTP client")
}

fn fetch(c: &reqwest::blocking::Client, url: &str) -> String {
    try_fetch(c, url).unwrap_or_else(|e| panic!("{e}"))
}

/// Non-panicking fetch: connection/TLS-level failures (observed against
/// fred.stlouisfed.org specifically — Akamai-fronted, likely a TLS
/// fingerprint mismatch between reqwest and curl, confirmed unrelated to
/// sandboxing) become an `Err` instead of a crash, same as a bad-JSON
/// response from `fetch_json_retry`.
fn try_fetch(c: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    let resp = c.get(url).send().map_err(|e| format!("request to {url} failed: {e}"))?;
    resp.text().map_err(|e| format!("reading body from {url} failed: {e}"))
}

/// fred.stlouisfed.org silently black-holes reqwest's TLS handshake until
/// timeout — reproduced identically from two different machines/networks,
/// while plain curl against the exact same URL succeeds in ~0.2s every
/// time. Akamai fingerprints the TLS client, not the request; shelling out
/// to curl for this one host is simpler and more honest than fighting it.
fn fetch_via_curl(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args(["-sSL", "--max-time", "20", url])
        .output()
        .map_err(|e| format!("failed to spawn curl for {url}: {e}"))?;
    if !output.status.success() {
        return Err(format!("curl exited {} for {url}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("curl output not utf8 for {url}: {e}"))
}

/// World Bank's edge intermittently answers `country/all` queries with a
/// WAF error page instead of JSON — reproduced directly with curl, no
/// pattern in per_page, unrelated to this client. A few retries with
/// backoff clear it in practice; callers decide whether a final failure
/// is fatal (`.expect(...)`) or just skips a non-critical update.
fn fetch_json_retry(
    c: &reqwest::blocking::Client,
    url: &str,
    tries: u32,
) -> Result<serde_json::Value, String> {
    let mut last_err = String::new();
    for attempt in 1..=tries {
        match serde_json::from_str::<serde_json::Value>(&fetch(c, url)) {
            Ok(v) => return Ok(v),
            Err(e) => {
                last_err = e.to_string();
                if attempt < tries {
                    eprintln!(
                        "warn: {url} did not parse as JSON (attempt {attempt}/{tries}: {e}) — retrying"
                    );
                    std::thread::sleep(Duration::from_millis(900 * attempt as u64));
                }
            }
        }
    }
    Err(format!(
        "giving up on {url} after {tries} attempts: {last_err}"
    ))
}

/// Read the value after `key = ` on its own line (still quoted, if a
/// string field) — mirrors the regex approach in the python script: only
/// the header fields are touched, [[visa_access]] blocks never parsed.
fn field<'a>(content: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key} = ");
    content
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("missing field `{key}`"))
        .trim()
}

fn unquote(s: &str) -> String {
    s.trim_matches('"').to_string()
}

/// Replace the value of `key = ...` in place, line by line — every other
/// byte in the file (including the [[visa_access]] arrays) is untouched.
fn set_field(content: &str, key: &str, new_value: &str) -> String {
    let prefix = format!("{key} = ");
    let mut out = String::with_capacity(content.len() + 16);
    for line in content.split_inclusive('\n') {
        let bare = line.trim_end_matches('\n');
        if bare.starts_with(&prefix) {
            out.push_str(&prefix);
            out.push_str(new_value);
        } else {
            out.push_str(bare);
        }
        if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Set `key`, inserting it right after `after_key`'s line if it doesn't
/// exist yet. Lets a brand-new `_prev` field get added to all 233
/// states/*.toml on its first run, then behave like `set_field` after.
fn upsert_field(content: &str, key: &str, after_key: &str, new_value: &str) -> String {
    let prefix = format!("{key} = ");
    if content.lines().any(|l| l.starts_with(&prefix)) {
        return set_field(content, key, new_value);
    }
    let after_prefix = format!("{after_key} = ");
    let mut out = String::with_capacity(content.len() + 32);
    for line in content.split_inclusive('\n') {
        out.push_str(line);
        if line.trim_end_matches('\n').starts_with(&after_prefix) {
            out.push_str(&prefix);
            out.push_str(new_value);
            out.push('\n');
        }
    }
    out
}

/// Replace the numeric literal in `pub const NAME: f64 = ...;`, keeping
/// any trailing `// comment` byte-for-byte.
fn set_const(content: &str, name: &str, formatted: &str) -> String {
    let prefix = format!("pub const {name}: f64 = ");
    let mut out = String::with_capacity(content.len() + 16);
    for line in content.split_inclusive('\n') {
        let bare = line.trim_end_matches('\n');
        if let Some(rest) = bare.strip_prefix(&prefix) {
            let tail = rest.find(';').map(|i| &rest[i..]).unwrap_or(";");
            out.push_str(&prefix);
            out.push_str(formatted);
            out.push_str(tail);
        } else {
            out.push_str(bare);
        }
        if line.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

fn fmt_cap(v: f64) -> String {
    if v >= 100.0 {
        format!("{v:.0}")
    } else if v >= 1.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}")
    }
}

fn commas(v: f64) -> String {
    let sign = if v < 0.0 { "-" } else { "" };
    let digits = (v.abs().round() as i64).to_string();
    let grouped: String = digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(",");
    format!("{sign}{grouped}")
}

struct Mover {
    delta: f64,
    code: String,
    cur: String,
    old: f64,
    new: f64,
    src: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_inserts_when_missing() {
        let toml = "money_supply_b_usd = 30682\ntoken_price_usd = 1.000000\nvisa_free_destinations = 186\n";
        let out = upsert_field(toml, "money_supply_b_usd_prev", "money_supply_b_usd", "30500");
        assert_eq!(
            out,
            "money_supply_b_usd = 30682\nmoney_supply_b_usd_prev = 30500\ntoken_price_usd = 1.000000\nvisa_free_destinations = 186\n"
        );
    }

    #[test]
    fn upsert_replaces_when_present() {
        let toml = "money_supply_b_usd = 30682\nmoney_supply_b_usd_prev = 30000\n";
        let out = upsert_field(toml, "money_supply_b_usd_prev", "money_supply_b_usd", "30500");
        assert_eq!(out, "money_supply_b_usd = 30682\nmoney_supply_b_usd_prev = 30500\n");
    }

    #[test]
    fn upsert_does_not_confuse_prefixed_key() {
        // "token_price_usd_prev" must not be matched by the
        // "token_price_usd = " prefix check used for the base field.
        let toml = "token_price_usd = 1.000000\n";
        let out = upsert_field(toml, "token_price_usd_prev", "token_price_usd", "0.999000");
        let out2 = set_field(&out, "token_price_usd", "1.010000");
        assert_eq!(out2, "token_price_usd = 1.010000\ntoken_price_usd_prev = 0.999000\n");
    }
}

fn main() {
    let api_key = std::env::var("EXCHANGERATE_API_KEY").unwrap_or_else(|_| {
        panic!(
            "set EXCHANGERATE_API_KEY before running this \
             (free key at https://www.exchangerate-api.com/ — never commit it, \
             export it in your shell instead)"
        )
    });
    let c = client();

    // --- FX --------------------------------------------------------
    let fx_url = format!("https://v6.exchangerate-api.com/v6/{api_key}/latest/USD");
    let fx = fetch_json_retry(&c, &fx_url, 3).expect("FX fetch");
    assert_eq!(
        fx["result"].as_str(),
        Some("success"),
        "FX fetch failed: {fx}"
    );
    let rates = fx["conversion_rates"]
        .as_object()
        .expect("no conversion_rates in FX response");
    let fx_date = fx["time_last_update_utc"].as_str().unwrap_or("unknown");

    // --- World Bank broad money (LCU) -------------------------------
    // Their `country/all` edge is observed to fail intermittently (and
    // sometimes for extended stretches) independent of this client —
    // reproduced with plain curl. A dead WB must not block the price
    // refresh for all 281 states: on failure, `wb` stays empty and every
    // state falls back to the same reval/kept path already used for any
    // single state missing from a healthy response.
    let mut wb: HashMap<String, (f64, i32)> = HashMap::new();
    match fetch_json_retry(&c, WB_URL, 5) {
        Err(e) => {
            eprintln!("warn: World Bank fetch failed ({e}); every state falls back to revalue")
        }
        Ok(wb_raw) => {
            if let Some(rows) = wb_raw.get(1).and_then(|v| v.as_array()) {
                for row in rows {
                    let Some(value) = row["value"].as_f64() else {
                        continue;
                    };
                    let Some(year) = row["date"].as_str().and_then(|d| d.parse::<i32>().ok())
                    else {
                        continue;
                    };
                    if year < MIN_YEAR {
                        continue;
                    }
                    let Some(id) = row["country"]["id"].as_str() else {
                        continue;
                    };
                    wb.insert(id.to_string(), (value, year));
                }
            }
        }
    }

    // --- US M2 from FRED --------------------------------------------
    let fred_row = fetch_via_curl(FRED_URL).ok().and_then(|csv| {
        csv.lines()
            .skip(1)
            .filter(|l| l.contains(','))
            .last()
            .and_then(|last| last.split_once(','))
            .and_then(|(date, val)| val.trim().parse::<f64>().ok().map(|v| (date.to_string(), v)))
    });
    match fred_row {
        Some((date, m2_b)) => {
            let year = date
                .get(0..4)
                .and_then(|y| y.parse::<i32>().ok())
                .unwrap_or(0);
            wb.insert("US".to_string(), (m2_b * 1e9, year));
        }
        None => eprintln!("warn: FRED M2 fetch failed or non-numeric; US falls back to revalue"),
    }

    // --- Numeraires: BTC / ETH / gold (PAXG = 1 troy oz) -------------
    match fetch_json_retry(&c, CG_URL, 3) {
        Ok(cg) => {
            let btc = cg["bitcoin"]["usd"].as_f64();
            let eth = cg["ethereum"]["usd"].as_f64();
            let xau = cg["pax-gold"]["usd"].as_f64();
            match (btc, eth, xau) {
                (Some(btc), Some(eth), Some(xau)) => {
                    let path = "src/numeraires.rs";
                    let mut src = fs::read_to_string(path).expect("read numeraires.rs");
                    if let Some(cny_rate) = rates.get("CNY").and_then(|v| v.as_f64()) {
                        if cny_rate > 0.0 {
                            src = set_const(&src, "CNY_USD", &format!("{:.6}", 1.0 / cny_rate));
                        }
                    }
                    src = set_const(&src, "BTC_USD", &format!("{btc:.1}"));
                    src = set_const(&src, "ETH_USD", &format!("{eth:.2}"));
                    src = set_const(&src, "XAU_USD", &format!("{xau:.2}"));
                    fs::write(path, src).expect("write numeraires.rs");
                    println!("numeraires: BTC ${btc:.0} ETH ${eth:.2} XAU ${xau:.2}");
                }
                _ => eprintln!(
                    "warn: CoinGecko response missing a field; src/numeraires.rs unchanged"
                ),
            }
        }
        Err(e) => eprintln!("warn: CoinGecko fetch failed ({e}); src/numeraires.rs unchanged"),
    }

    // --- rewrite tomls ------------------------------------------------
    let mut paths: Vec<PathBuf> = fs::read_dir("states")
        .expect("states/ dir not found — run from repo root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false))
        .collect();
    paths.sort();

    let (mut n_wb, mut n_reval, mut n_kept) = (0u32, 0u32, 0u32);
    let mut no_fx: Vec<String> = Vec::new();
    let mut movers: Vec<Mover> = Vec::new();

    for path in &paths {
        let content = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let code = unquote(field(&content, "code"));
        let cur = unquote(field(&content, "currency_code"));
        let old_cap: f64 = field(&content, "money_supply_b_usd")
            .parse()
            .unwrap_or_else(|e| panic!("{code}: bad money_supply_b_usd: {e}"));
        let old_price: f64 = field(&content, "token_price_usd")
            .parse()
            .unwrap_or_else(|e| panic!("{code}: bad token_price_usd: {e}"));

        let live_rate = rates
            .get(&cur)
            .and_then(|v| v.as_f64())
            .filter(|&r| r > 0.0);
        let new_price = match live_rate {
            Some(r) => 1.0 / r,
            None => {
                no_fx.push(format!("{code}:{cur}"));
                old_price
            }
        };

        let (new_cap, src) = if let Some(&(lcu, year)) = wb.get(&code) {
            n_wb += 1;
            (lcu * new_price / 1e9, format!("WB{year}"))
        } else if old_price > 0.0 {
            let ratio = new_price / old_price;
            if !(1.0 / 3.0..=3.0).contains(&ratio) {
                // a >3x FX jump without fresh supply data means
                // redenomination or a broken old quote — revaluing units
                // is invalid, keep the cap
                n_kept += 1;
                (old_cap, "kept:fx-jump".to_string())
            } else {
                n_reval += 1;
                (old_cap * ratio, "reval".to_string())
            }
        } else {
            n_kept += 1;
            (old_cap, "kept".to_string())
        };

        if old_cap > 1.0 {
            movers.push(Mover {
                delta: (new_cap / old_cap - 1.0).abs(),
                code: code.clone(),
                cur: cur.clone(),
                old: old_cap,
                new: new_cap,
                src: src.clone(),
            });
        }

        // snapshot today's (about-to-be-old) values as "prev" before
        // overwriting, so the site can show a day-over-day delta
        let updated = upsert_field(
            &content,
            "token_price_usd_prev",
            "token_price_usd",
            &format!("{old_price:.6}"),
        );
        let updated = upsert_field(
            &updated,
            "money_supply_b_usd_prev",
            "money_supply_b_usd",
            &fmt_cap(old_cap),
        );
        let updated = set_field(&updated, "token_price_usd", &format!("{new_price:.6}"));
        let updated = set_field(&updated, "money_supply_b_usd", &fmt_cap(new_cap));
        fs::write(path, updated).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
    }

    println!("FX as of: {fx_date}");
    println!("states: WB-updated={n_wb}  revalued={n_reval}  kept={n_kept}");
    println!(
        "no FX rate (price kept): {}",
        if no_fx.is_empty() {
            "none".to_string()
        } else {
            no_fx.join(", ")
        }
    );
    println!("\nbiggest cap movers:");
    movers.sort_by(|a, b| b.delta.partial_cmp(&a.delta).unwrap());
    for m in movers.iter().take(15) {
        let pct = (m.new / m.old - 1.0) * 100.0;
        println!(
            "  {} {}: ${}B -> ${}B  ({:+.0}%, {})",
            m.code,
            m.cur,
            commas(m.old),
            commas(m.new),
            pct,
            m.src
        );
    }
}
