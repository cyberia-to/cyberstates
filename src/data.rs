use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VisaAccess {
    pub country: String,
    #[serde(rename = "type")]
    pub access_type: String,
    pub days: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Country {
    pub name: String,
    pub code: String,
    /// Canonical URL segment: /state/{slug}, /from/{slug}/to/{slug}
    pub slug: String,
    pub flag: String,
    pub region: String,
    pub population: u64,
    pub land_area_km2: u64,
    pub currency_code: String,
    pub currency_name: String,
    pub money_supply_b_usd: f64,
    pub token_price_usd: f64,
    /// Yesterday's snapshot — 0.0 for any state the daily updater hasn't
    /// touched yet, in which case deltas render as flat/hidden.
    pub money_supply_b_usd_prev: f64,
    pub token_price_usd_prev: f64,
    pub visa_free_destinations: u32,
    pub visa_free_inbound: u32,
    pub oceans: String,
}

impl Country {
    /// Canonical path for this state page.
    pub fn path(&self) -> String {
        format!("/state/{}", self.slug)
    }
}

/// Resolve a path segment: prefer slug match, fall back to ISO/code (legacy URLs).
pub fn find_country(param: &str) -> Option<Country> {
    let countries = load_countries();
    let key = param.to_lowercase();
    countries
        .iter()
        .find(|c| c.slug == key)
        .or_else(|| {
            let upper = param.to_uppercase();
            countries.iter().find(|c| c.code == upper)
        })
        .cloned()
}

/// Look up slug for a map path id / state code (SVG uses ISO-style codes).
pub fn slug_for_code(code: &str) -> Option<String> {
    let upper = code.to_uppercase();
    load_countries()
        .iter()
        .find(|c| c.code == upper)
        .map(|c| c.slug.clone())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CountryIndex {
    pub eco_out_pct: f64, // weighted % of world economy accessible
    pub eco_in_pct: f64,  // weighted % of world economy that can visit
    pub pop_out_pct: f64, // weighted % of world population accessible
    pub pop_in_pct: f64,  // weighted % of world population that can visit
    pub freedom: f64,     // √(eco_out × pop_out)
    pub openness: f64,    // √(eco_in × pop_in)
}

include!(concat!(env!("OUT_DIR"), "/countries.rs"));

static VISA_DATA: OnceLock<HashMap<String, Vec<VisaAccess>>> = OnceLock::new();

pub fn get_visa_data() -> &'static HashMap<String, Vec<VisaAccess>> {
    VISA_DATA.get_or_init(|| serde_json::from_str(VISA_DATA_JSON).unwrap_or_default())
}

pub fn get_visa_outgoing(code: &str) -> Vec<VisaAccess> {
    get_visa_data().get(code).cloned().unwrap_or_default()
}

pub fn get_visa_incoming(country_name: &str) -> Vec<VisaAccess> {
    let data = get_visa_data();
    let countries = load_countries();
    let mut results = Vec::new();

    for (code, entries) in data.iter() {
        if let Some(entry) = entries.iter().find(|e| e.country == country_name) {
            let holder_name = countries
                .iter()
                .find(|c| c.code == code.to_uppercase())
                .map(|c| c.name.clone())
                .unwrap_or_else(|| code.to_uppercase());
            results.push(VisaAccess {
                country: holder_name,
                access_type: entry.access_type.clone(),
                days: entry.days,
            });
        }
    }

    results.sort_by(|a, b| {
        a.access_type
            .cmp(&b.access_type)
            .then(a.country.cmp(&b.country))
    });
    results
}

pub fn access_type_weight(t: &str) -> f64 {
    match t {
        "visa-free" => 1.0,
        "visa-on-arrival" => 0.8,
        "eta" | "e-visa" => 0.5,
        "visa-required" => 0.1,
        "no-admission" => 0.0,
        _ => 0.0,
    }
}

pub fn access_type_color(t: &str) -> &'static str {
    match t {
        "visa-free" => "var(--cyber-green)",
        "visa-on-arrival" => "var(--cyber-cyan)",
        "eta" | "e-visa" => "var(--cyber-yellow)",
        "visa-required" => "var(--cyber-red)",
        "no-admission" => "#555",
        _ => "#666",
    }
}

pub fn access_type_label(t: &str) -> &'static str {
    match t {
        "visa-free" => "FREE",
        "visa-on-arrival" => "ARRIVAL",
        "eta" | "e-visa" => "ONLINE",
        "visa-required" => "INTERVIEW",
        "no-admission" => "DENIED",
        _ => "UNKNOWN",
    }
}

/// Aggregate token info from all countries sharing the same currency
#[derive(Clone, Debug)]
pub struct Token {
    pub code: String,
    pub name: String,
    pub price_usd: f64,
    pub price_usd_prev: f64,
    pub total_supply_b_usd: f64,
    pub total_supply_b_usd_prev: f64,
    pub total_population: u64,
    pub total_area_km2: u64,
    pub countries: Vec<(String, String, String)>, // (country_code, country_name, flag)
}

pub fn get_tokens() -> Vec<Token> {
    let countries = load_countries();
    let mut map: HashMap<String, Token> = HashMap::new();

    for c in &countries {
        let entry = map.entry(c.currency_code.clone()).or_insert_with(|| Token {
            code: c.currency_code.clone(),
            name: c.currency_name.clone(),
            price_usd: c.token_price_usd,
            price_usd_prev: c.token_price_usd_prev,
            total_supply_b_usd: 0.0,
            total_supply_b_usd_prev: 0.0,
            total_population: 0,
            total_area_km2: 0,
            countries: Vec::new(),
        });
        entry.total_supply_b_usd += c.money_supply_b_usd;
        entry.total_supply_b_usd_prev += c.money_supply_b_usd_prev;
        entry.total_population += c.population;
        entry.total_area_km2 += c.land_area_km2;
        entry
            .countries
            .push((c.code.clone(), c.name.clone(), c.flag.clone()));
        if entry.price_usd <= 0.0 && c.token_price_usd > 0.0 {
            entry.price_usd = c.token_price_usd;
            entry.price_usd_prev = c.token_price_usd_prev;
        }
    }

    let mut tokens: Vec<Token> = map.into_values().collect();
    tokens.sort_by(|a, b| {
        b.total_supply_b_usd
            .partial_cmp(&a.total_supply_b_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    tokens
}

pub fn get_token(code: &str) -> Option<Token> {
    get_tokens()
        .into_iter()
        .find(|t| t.code.to_uppercase() == code.to_uppercase())
}

impl Country {
    pub fn population_fmt(&self) -> String {
        compact_count(self.population as f64)
    }

    pub fn land_area_fmt(&self) -> String {
        format!("{} km²", format_area(self.land_area_km2))
    }

    /// Cap in USD (money_supply_b_usd)
    pub fn cap_fmt(&self) -> String {
        fmt_usd_billions(self.money_supply_b_usd)
    }

    /// Supply in native tokens = cap / price, scaled k/M/B/T/Q — a
    /// 143-million-token supply must read 143M, never round to 0B.
    pub fn supply_fmt(&self) -> String {
        if self.token_price_usd <= 0.0 {
            return "N/A".to_string();
        }
        let tokens = self.money_supply_b_usd * 1e9 / self.token_price_usd;
        if tokens <= 0.0 {
            return "N/A".to_string();
        }
        let sig = |v: f64| {
            if v >= 100.0 {
                format!("{:.0}", v)
            } else if v >= 10.0 {
                format!("{:.1}", v)
            } else {
                format!("{:.2}", v)
            }
        };
        let (v, s) = if tokens >= 1e15 {
            (tokens / 1e15, "Q")
        } else if tokens >= 1e12 {
            (tokens / 1e12, "T")
        } else if tokens >= 1e9 {
            (tokens / 1e9, "B")
        } else if tokens >= 1e6 {
            (tokens / 1e6, "M")
        } else if tokens >= 1e3 {
            (tokens / 1e3, "k")
        } else {
            (tokens, "")
        };
        format!("{}{}", sig(v), s)
    }

    pub fn price_fmt(&self) -> String {
        if self.token_price_usd <= 0.0 {
            "N/A".to_string()
        } else if self.token_price_usd >= 1.0 {
            format!("${:.2}", self.token_price_usd)
        } else if self.token_price_usd >= 0.01 {
            format!("${:.4}", self.token_price_usd)
        } else {
            format!("${:.6}", self.token_price_usd)
        }
    }

    /// Day-over-day change in CAPITAL, as a fraction (0.01 = +1%). `None`
    /// until the daily updater has written a prior snapshot for this state.
    pub fn cap_delta(&self) -> Option<f64> {
        delta_pct(self.money_supply_b_usd, self.money_supply_b_usd_prev)
    }

    /// Day-over-day change in token PRICE, as a fraction.
    pub fn price_delta(&self) -> Option<f64> {
        delta_pct(self.token_price_usd, self.token_price_usd_prev)
    }

    /// Weighted index: economy and population reach, both directions
    /// Value of the rating object for this state, in the field's raw unit
    /// (cap in B USD, human/land in USD, scores 0-100, counts raw).
    pub fn metric(&self, f: SortField) -> f64 {
        match f {
            SortField::Capital => self.money_supply_b_usd,
            // growth today: only gainers score; flat/down/missing → −∞ (bottom).
            SortField::Growth => self
                .price_delta()
                .filter(|d| *d > 0.0005)
                .map(|d| d * 100.0)
                .unwrap_or(f64::NEG_INFINITY),
            // loss today: magnitude of drop so biggest losers rank #1
            // (descending). gainers/flat/missing → −∞.
            SortField::Loss => self
                .price_delta()
                .filter(|d| *d < -0.0005)
                .map(|d| -d * 100.0)
                .unwrap_or(f64::NEG_INFINITY),
            SortField::Human => {
                if self.population > 0 {
                    self.money_supply_b_usd * 1e9 / self.population as f64
                } else {
                    0.0
                }
            }
            SortField::Land => {
                if self.land_area_km2 > 0 {
                    self.money_supply_b_usd * 1e9 / self.land_area_km2 as f64
                } else {
                    0.0
                }
            }
            SortField::Freedom => self.index().freedom,
            SortField::Hospitality => self.index().openness,
            SortField::Population => self.population as f64,
            SortField::Territory => self.land_area_km2 as f64,
            SortField::Density => {
                if self.land_area_km2 > 0 {
                    self.population as f64 / self.land_area_km2 as f64
                } else {
                    0.0
                }
            }
        }
    }

    /// Weights: visa-free=1.0, VoA=0.8, eta/e-visa=0.5, visa-required=0.1, no-admission=0.0
    pub fn index(&self) -> CountryIndex {
        index_cache()
            .get(self.code.as_str())
            .copied()
            .unwrap_or_default()
    }
}

static INDEX_CACHE: OnceLock<HashMap<String, CountryIndex>> = OnceLock::new();

/// All freedom/hospitality scores in ONE pass over the visa matrix.
/// The naive per-state walk was O(states x matrix) with linear lookups,
/// and sorting called it per comparison — seconds of lag per click.
fn index_cache() -> &'static HashMap<String, CountryIndex> {
    INDEX_CACHE.get_or_init(|| {
        let countries = load_countries();
        let data = get_visa_data();

        // world totals are terrestrial: celestial listings (Earth the
        // body among them) must not dilute the visa index denominators
        let total_cap: f64 = countries
            .iter()
            .filter(|c| is_terrestrial(&c.region) && !is_aggregate(&c.code))
            .map(|c| c.money_supply_b_usd)
            .sum();
        let total_pop: f64 = countries
            .iter()
            .filter(|c| is_terrestrial(&c.region) && !is_aggregate(&c.code))
            .map(|c| c.population as f64)
            .sum();

        let by_name: HashMap<&str, &Country> =
            countries.iter().map(|c| (c.name.as_str(), c)).collect();
        let by_code: HashMap<String, &Country> = countries
            .iter()
            .map(|c| (c.code.to_uppercase(), c))
            .collect();

        // (eco_out, pop_out, eco_in, pop_in) accumulated per state code
        let mut acc: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
        for (code, entries) in data.iter() {
            let Some(holder) = by_code.get(&code.to_uppercase()) else {
                continue;
            };
            for e in entries {
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

        countries
            .iter()
            .map(|c| {
                let (eco_out, pop_out, eco_in, pop_in) =
                    acc.get(&c.code).copied().unwrap_or_default();
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
                    CountryIndex {
                        eco_out_pct,
                        eco_in_pct,
                        pop_out_pct,
                        pop_in_pct,
                        freedom: (eco_out_pct * pop_out_pct).sqrt(),
                        openness: (eco_in_pct * pop_in_pct).sqrt(),
                    },
                )
            })
            .collect()
    })
}

/// `None` when there's no usable prior value to compare against (0.0 =
/// state not yet touched by the daily updater, or a redenomination guard
/// kept the old figure — either way a delta would be meaningless noise).
fn delta_pct(current: f64, prev: f64) -> Option<f64> {
    if prev > 0.0 && current > 0.0 {
        Some(current / prev - 1.0)
    } else {
        None
    }
}

/// Small ▲/▼ delta badge shared by every stat card that shows one — the
/// dead-zone (< 0.05%) reads as flat rather than a misleading sliver arrow.
pub fn delta_badge(pct: f64) -> (&'static str, String, &'static str) {
    if pct > 0.0005 {
        ("▲", format!("{:.1}%", pct * 100.0), "var(--cyber-green)")
    } else if pct < -0.0005 {
        ("▼", format!("{:.1}%", -pct * 100.0), "var(--cyber-red)")
    } else {
        ("•", "0.0%".to_string(), "#777")
    }
}

/// One-cell rendering of a delta, for table columns — a plain (text, color)
/// pair matching the shape `metric_cell()` already returns everywhere else.
/// `None` (no prior snapshot yet) reads as a dim dash, not a fake 0.0%.
pub fn delta_cell(pct: Option<f64>) -> (String, &'static str) {
    match pct {
        None => ("—".to_string(), "#333"),
        Some(pct) => {
            let (arrow, text, color) = delta_badge(pct);
            (format!("{arrow}{text}"), color)
        }
    }
}

fn fmt_usd_billions(v: f64) -> String {
    if v <= 0.0 {
        "N/A".to_string()
    } else if v >= 1000.0 {
        format!("${:.1}T", v / 1000.0)
    } else {
        format!("${:.0}B", v)
    }
}

impl Token {
    pub fn cap_fmt(&self) -> String {
        fmt_usd_billions(self.total_supply_b_usd)
    }

    pub fn cap_delta(&self) -> Option<f64> {
        delta_pct(self.total_supply_b_usd, self.total_supply_b_usd_prev)
    }

    pub fn price_delta(&self) -> Option<f64> {
        delta_pct(self.price_usd, self.price_usd_prev)
    }

    pub fn supply(&self) -> f64 {
        if self.price_usd > 0.0 {
            self.total_supply_b_usd / self.price_usd
        } else {
            0.0
        }
    }

    pub fn supply_fmt(&self) -> String {
        let s = self.supply();
        if s <= 0.0 {
            "N/A".to_string()
        } else if s >= 1_000_000.0 {
            format!("{:.1}Q", s / 1_000_000.0)
        } else if s >= 1000.0 {
            format!("{:.0}T", s / 1000.0)
        } else if s >= 1.0 {
            format!("{:.0}B", s)
        } else {
            format!("{:.1}B", s)
        }
    }

    pub fn price_fmt(&self) -> String {
        if self.price_usd <= 0.0 {
            "N/A".to_string()
        } else if self.price_usd >= 1.0 {
            format!("${:.2}", self.price_usd)
        } else if self.price_usd >= 0.01 {
            format!("${:.4}", self.price_usd)
        } else {
            format!("${:.6}", self.price_usd)
        }
    }
}

/// One compact scale for every count on the site — population and
/// territory alike read in k/M/B/T/Q: 8.03B people, 87.0M km², 11.3k.
/// Precision tapers with magnitude (2 / 1 / 0 decimals) so figures stay
/// comparable down a column; anything under 1,000 prints whole.
pub fn compact_count(v: f64) -> String {
    if v <= 0.0 {
        return "0".to_string();
    }
    let (val, suffix) = if v >= 1e15 {
        (v / 1e15, "Q")
    } else if v >= 1e12 {
        (v / 1e12, "T")
    } else if v >= 1e9 {
        (v / 1e9, "B")
    } else if v >= 1e6 {
        (v / 1e6, "M")
    } else if v >= 1e3 {
        (v / 1e3, "k")
    } else {
        return format!("{}", v.round() as u64);
    };
    let sig = if val >= 100.0 {
        format!("{:.0}", val)
    } else if val >= 10.0 {
        format!("{:.1}", val)
    } else {
        format!("{:.2}", val)
    };
    format!("{}{}", sig, suffix)
}

pub fn format_area(v: u64) -> String {
    compact_count(v as f64)
}

/// Listing class for the header toggles. Most rows are exactly one class;
/// Antarctica is dual — both continent and country — so either toggle
/// keeps it on the board.
#[derive(Clone, Copy, PartialEq)]
pub enum ListingClass {
    Planet,
    Continent,
    Country,
}

/// The macro-region aggregates: one listing per continent-scale sum.
/// Antarctica (AQ) is continent-class too, but a single body, not a sum
/// of member states — and it is also a country (dual membership).
pub const AGGREGATES: &[&str] = &["OCNA", "AFRI", "EURA", "AMER"];

/// Which market-regions an aggregate sums over. Eurasia is one ground:
/// Europe, Asia and the Middle East share the landmass, so they share
/// the listing.
pub fn aggregate_regions(code: &str) -> &'static [&'static str] {
    match code {
        "OCNA" => &["Oceania"],
        "AFRI" => &["Africa"],
        "EURA" => &["Asia", "Europe", "Eurasia", "Middle East"],
        "AMER" => &["North America", "Latin America"],
        _ => &[],
    }
}

pub fn is_aggregate(code: &str) -> bool {
    AGGREGATES.contains(&code)
}

impl Country {
    /// True if this row belongs to the given listing class. Antarctica
    /// belongs to both Continent and Country; everything else is exclusive.
    pub fn belongs_to(&self, class: ListingClass) -> bool {
        match class {
            ListingClass::Planet => self.region == "Solar System",
            ListingClass::Continent => self.code == "AQ" || is_aggregate(&self.code),
            // countries + Antarctica; not planets, not macro-aggregates
            ListingClass::Country => self.region != "Solar System" && !is_aggregate(&self.code),
        }
    }

    /// Visible under the header toggles if it matches *any* enabled class.
    /// AQ shows when countries are on, when continents are on, or both —
    /// never duplicated (one row either way).
    pub fn class_visible(&self, planets: bool, continents: bool, countries: bool) -> bool {
        (planets && self.belongs_to(ListingClass::Planet))
            || (continents && self.belongs_to(ListingClass::Continent))
            || (countries && self.belongs_to(ListingClass::Country))
    }

    /// Primary class for single-band callers (map planet spectrum, etc.).
    /// Dual-class Antarctica prefers Country — it ranks with the terrestrial
    /// roll; continent toggle still catches it via `belongs_to`.
    pub fn class(&self) -> ListingClass {
        if self.belongs_to(ListingClass::Planet) {
            ListingClass::Planet
        } else if is_aggregate(&self.code) {
            ListingClass::Continent
        } else {
            ListingClass::Country
        }
    }
}

/// Default class filter: countries only. Planets and continents start off.
pub const DEFAULT_CLASSES: (bool, bool, bool) = (false, false, true); // planets, continents, countries

const CLASS_FILTER_KEY: &str = "class_filter";

fn encode_classes(c: (bool, bool, bool)) -> String {
    let mut on: Vec<&str> = Vec::new();
    if c.0 {
        on.push("planets");
    }
    if c.1 {
        on.push("continents");
    }
    if c.2 {
        on.push("countries");
    }
    on.join(",")
}

fn decode_classes(v: &str) -> (bool, bool, bool) {
    let has = |s: &str| v.split(',').filter(|x| !x.is_empty()).any(|x| x == s);
    (has("planets"), has("continents"), has("countries"))
}

/// Global site setting — same toggles on states and tokens (localStorage).
pub fn load_class_filter() -> (bool, bool, bool) {
    #[cfg(target_arch = "wasm32")]
    {
        return web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|ls| ls.get_item(CLASS_FILTER_KEY).ok().flatten())
            .map(|s| decode_classes(&s))
            .unwrap_or(DEFAULT_CLASSES);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        DEFAULT_CLASSES
    }
}

pub fn store_class_filter(c: (bool, bool, bool)) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = ls.set_item(CLASS_FILTER_KEY, &encode_classes(c));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = c;
    }
}

/// Token is visible if any member state matches the class toggles.
pub fn token_class_visible(
    t: &Token,
    by_code: &HashMap<String, &Country>,
    planets: bool,
    continents: bool,
    countries: bool,
) -> bool {
    t.countries.iter().any(|(code, _, _)| {
        by_code
            .get(code.as_str())
            .map(|c| c.class_visible(planets, continents, countries))
            .unwrap_or(false)
    })
}

/// Terrestrial = carries real population and capital; the visa index
/// denominators sum over these only, so celestial and oceanic listings
/// never dilute the scores.
pub fn is_terrestrial(region: &str) -> bool {
    !matches!(region, "Oceans" | "Terra Nullius" | "Solar System")
}

pub const REGIONS: &[&str] = &[
    "All",
    "Africa",
    "Asia",
    "Europe",
    "Eurasia",
    "Latin America",
    "Middle East",
    "North America",
    "Oceania",
    "Antarctica",
    "Terra Nullius",
    "Solar System",
];

#[derive(Clone, Copy, PartialEq)]
pub enum SortField {
    Capital,
    /// today's token price gain (%). ranks like CAPITAL ranks by stock size.
    Growth,
    /// today's token price loss magnitude (%). biggest losers first.
    Loss,
    Human,
    Land,
    Freedom,
    Hospitality,
    Population,
    Territory,
    Density,
}

/// Podium metal for a rank — gold, silver, bronze — None below the podium.
pub fn rank_medal(rank: usize) -> Option<&'static str> {
    match rank {
        1 => Some("#ffd700"),
        2 => Some("#c0c4cc"),
        3 => Some("#cd7f32"),
        _ => None,
    }
}

/// Rank column presentation, shared by every table: the podium wears its
/// metal, the top ten stays readable, the field stays quiet.
pub fn rank_color(rank: usize) -> &'static str {
    rank_medal(rank).unwrap_or(if rank <= 10 { "#777" } else { "#3d3d3d" })
}

pub fn rank_weight(rank: usize) -> &'static str {
    if rank <= 3 {
        "700"
    } else {
        "400"
    }
}

impl SortField {
    /// Ratings where a LOWER value is the good end — map colors invert
    /// so green always means "better".
    pub fn lower_is_better(&self) -> bool {
        matches!(self, Self::Density)
    }

    /// Compact header for the metric column / rating button.
    pub fn short(&self) -> &'static str {
        match self {
            Self::Capital => "CAPITAL",
            // full phrase — user-facing name, not "24H"
            Self::Growth => "GROWTH TODAY",
            Self::Loss => "LOSS TODAY",
            Self::Human => "CITIZEN",
            Self::Land => "LAND",
            Self::Freedom => "FREEDOM",
            Self::Hospitality => "HOSPITALITY",
            Self::Population => "POPULATION",
            Self::Territory => "TERRITORY",
            Self::Density => "DENSITY",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Capital => "CAPITAL",
            Self::Growth => "GROWTH TODAY",
            Self::Loss => "LOSS TODAY",
            Self::Human => "CITIZEN VALUE",
            Self::Land => "LAND VALUE",
            Self::Freedom => "TRAVEL FREEDOM",
            Self::Hospitality => "HOSPITALITY",
            Self::Population => "POPULATION",
            Self::Territory => "TERRITORY",
            Self::Density => "DENSITY",
        }
    }

    /// True when the rating *is* the day change — the fixed PRICE Δ column
    /// would duplicate the metric column, so the table hides it.
    pub fn is_day_change(&self) -> bool {
        matches!(self, Self::Growth | Self::Loss)
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Capital => "capital",
            Self::Growth => "growth",
            Self::Loss => "loss",
            Self::Human => "citizen-value",
            Self::Land => "land-value",
            Self::Freedom => "travel-freedom",
            Self::Hospitality => "hospitality",
            Self::Population => "population",
            Self::Territory => "territory",
            Self::Density => "density",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Some(match s {
            "capital" | "cap" => Self::Capital,
            "growth" | "growth-today" | "gain" | "24h" => Self::Growth,
            "loss" | "loss-today" | "losers" => Self::Loss,
            "citizen-value" | "citizen" | "human-value" | "human" => Self::Human,
            "land-value" | "land" => Self::Land,
            "travel-freedom" | "freedom" => Self::Freedom,
            "hospitality" | "openness" => Self::Hospitality,
            "population" => Self::Population,
            "territory" | "area" => Self::Territory,
            "density" => Self::Density,
            _ => return None,
        })
    }

    /// The ratings, in display order.
    // capital · day tape (gain/loss) · stocks · derived rates · freedom scores
    pub const ALL: [SortField; 10] = [
        Self::Capital,
        Self::Growth,
        Self::Loss,
        Self::Population,
        Self::Territory,
        Self::Human,
        Self::Land,
        Self::Density,
        Self::Freedom,
        Self::Hospitality,
    ];

    /// True where a derived group begins — the pill rows draw a dot here.
    pub fn derived_break(&self) -> bool {
        matches!(self, Self::Population | Self::Human | Self::Freedom)
    }
}
