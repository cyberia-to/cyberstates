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
    pub flag: String,
    pub region: String,
    pub population: u64,
    pub land_area_km2: u64,
    pub currency_code: String,
    pub currency_name: String,
    pub money_supply_b_usd: f64,
    pub token_price_usd: f64,
    pub visa_free_destinations: u32,
    pub visa_free_inbound: u32,
}

#[derive(Clone, Debug)]
pub struct CountryIndex {
    pub eco_out_pct: f64,  // weighted % of world economy accessible
    pub eco_in_pct: f64,   // weighted % of world economy that can visit
    pub pop_out_pct: f64,  // weighted % of world population accessible
    pub pop_in_pct: f64,   // weighted % of world population that can visit
    pub freedom: f64,      // √(eco_out × pop_out)
    pub openness: f64,     // √(eco_in × pop_in)
}

include!(concat!(env!("OUT_DIR"), "/countries.rs"));

static VISA_DATA: OnceLock<HashMap<String, Vec<VisaAccess>>> = OnceLock::new();

pub fn get_visa_data() -> &'static HashMap<String, Vec<VisaAccess>> {
    VISA_DATA.get_or_init(|| {
        serde_json::from_str(VISA_DATA_JSON).unwrap_or_default()
    })
}

pub fn get_visa_outgoing(code: &str) -> Vec<VisaAccess> {
    get_visa_data()
        .get(code)
        .cloned()
        .unwrap_or_default()
}

pub fn get_visa_incoming(country_name: &str) -> Vec<VisaAccess> {
    let data = get_visa_data();
    let countries = load_countries();
    let mut results = Vec::new();

    for (code, entries) in data.iter() {
        if let Some(entry) = entries.iter().find(|e| e.country == country_name) {
            let holder_name = countries.iter()
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

    results.sort_by(|a, b| a.access_type.cmp(&b.access_type).then(a.country.cmp(&b.country)));
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
    pub total_supply_b_usd: f64,
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
            total_supply_b_usd: 0.0,
            total_population: 0,
            total_area_km2: 0,
            countries: Vec::new(),
        });
        entry.total_supply_b_usd += c.money_supply_b_usd;
        entry.total_population += c.population;
        entry.total_area_km2 += c.land_area_km2;
        entry.countries.push((c.code.clone(), c.name.clone(), c.flag.clone()));
        if entry.price_usd <= 0.0 && c.token_price_usd > 0.0 {
            entry.price_usd = c.token_price_usd;
        }
    }

    let mut tokens: Vec<Token> = map.into_values().collect();
    tokens.sort_by(|a, b| b.total_supply_b_usd.partial_cmp(&a.total_supply_b_usd).unwrap_or(std::cmp::Ordering::Equal));
    tokens
}

pub fn get_token(code: &str) -> Option<Token> {
    get_tokens().into_iter().find(|t| t.code.to_uppercase() == code.to_uppercase())
}

impl Country {
    pub fn population_fmt(&self) -> String {
        format_number(self.population)
    }

    pub fn land_area_fmt(&self) -> String {
        format!("{} km²", format_number(self.land_area_km2))
    }

    /// Cap in USD (money_supply_b_usd)
    pub fn cap_fmt(&self) -> String {
        fmt_usd_billions(self.money_supply_b_usd)
    }

    /// Supply in native tokens = cap / price, scaled B/T/Q.
    pub fn supply_fmt(&self) -> String {
        if self.token_price_usd <= 0.0 {
            return "N/A".to_string();
        }
        let s = self.money_supply_b_usd / self.token_price_usd;
        if s <= 0.0 {
            "N/A".to_string()
        } else if s >= 1_000_000.0 {
            format!("{:.1}Q", s / 1_000_000.0)
        } else if s >= 1000.0 {
            format!("{:.0}T", s / 1000.0)
        } else {
            format!("{:.0}B", s)
        }
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

    /// Weighted index: economy and population reach, both directions
    /// Value of the rating object for this state, in the field's raw unit
    /// (cap in B USD, human/land in USD, scores 0-100, counts raw).
    pub fn metric(&self, f: SortField) -> f64 {
        match f {
            SortField::Capital => self.money_supply_b_usd,
            SortField::Human => {
                if self.population > 0 { self.money_supply_b_usd * 1e9 / self.population as f64 } else { 0.0 }
            }
            SortField::Land => {
                if self.land_area_km2 > 0 { self.money_supply_b_usd * 1e9 / self.land_area_km2 as f64 } else { 0.0 }
            }
            SortField::Freedom => self.index().freedom,
            SortField::Hospitality => self.index().openness,
            SortField::Population => self.population as f64,
            SortField::Territory => self.land_area_km2 as f64,
            SortField::Density => {
                if self.land_area_km2 > 0 { self.population as f64 / self.land_area_km2 as f64 } else { 0.0 }
            }
        }
    }

    /// Weights: visa-free=1.0, VoA=0.8, eta/e-visa=0.5, visa-required=0.1, no-admission=0.0
    pub fn index(&self) -> CountryIndex {
        let countries = load_countries();
        let data = get_visa_data();

        let total_cap: f64 = countries.iter().map(|c| c.money_supply_b_usd).sum();
        let total_pop: f64 = countries.iter().map(|c| c.population as f64).sum();

        let by_name: std::collections::HashMap<&str, &Country> =
            countries.iter().map(|c| (c.name.as_str(), c)).collect();

        // Outgoing — weighted sum of destinations
        let outgoing = get_visa_outgoing(&self.code);
        let mut eco_out: f64 = 0.0;
        let mut pop_out: f64 = 0.0;
        for e in &outgoing {
            let w = access_type_weight(&e.access_type);
            if let Some(dest) = by_name.get(e.country.as_str()) {
                eco_out += w * dest.money_supply_b_usd;
                pop_out += w * dest.population as f64;
            }
        }

        // Incoming — weighted sum of visitors
        let mut eco_in: f64 = 0.0;
        let mut pop_in: f64 = 0.0;
        for (code, entries) in data.iter() {
            if let Some(entry) = entries.iter().find(|e| e.country == self.name) {
                let w = access_type_weight(&entry.access_type);
                if let Some(holder) = countries.iter().find(|c| c.code == code.to_uppercase()) {
                    eco_in += w * holder.money_supply_b_usd;
                    pop_in += w * holder.population as f64;
                }
            }
        }

        let eco_out_pct = if total_cap > 0.0 { eco_out / total_cap * 100.0 } else { 0.0 };
        let eco_in_pct = if total_cap > 0.0 { eco_in / total_cap * 100.0 } else { 0.0 };
        let pop_out_pct = if total_pop > 0.0 { pop_out / total_pop * 100.0 } else { 0.0 };
        let pop_in_pct = if total_pop > 0.0 { pop_in / total_pop * 100.0 } else { 0.0 };

        let freedom = (eco_out_pct * pop_out_pct).sqrt();
        let openness = (eco_in_pct * pop_in_pct).sqrt();

        CountryIndex { eco_out_pct, eco_in_pct, pop_out_pct, pop_in_pct, freedom, openness }
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

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
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
];

#[derive(Clone, Copy, PartialEq)]
pub enum SortField {
    Capital,
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
    if rank <= 3 { "700" } else { "400" }
}

impl SortField {
    /// Ratings where a LOWER value is the good end — map colors invert
    /// so green always means "better".
    pub fn lower_is_better(&self) -> bool {
        matches!(self, Self::Density)
    }

    /// One-word header for the narrow table column — never wraps.
    pub fn short(&self) -> &'static str {
        match self {
            Self::Capital => "CAPITAL",
            Self::Human => "HUMAN",
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
            Self::Human => "HUMAN VALUE",
            Self::Land => "LAND VALUE",
            Self::Freedom => "TRAVEL FREEDOM",
            Self::Hospitality => "HOSPITALITY",
            Self::Population => "POPULATION",
            Self::Territory => "TERRITORY",
            Self::Density => "DENSITY",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Capital => "capital",
            Self::Human => "human-value",
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
            "human-value" | "human" => Self::Human,
            "land-value" | "land" => Self::Land,
            "travel-freedom" | "freedom" => Self::Freedom,
            "hospitality" | "openness" => Self::Hospitality,
            "population" => Self::Population,
            "territory" | "area" => Self::Territory,
            "density" => Self::Density,
            _ => return None,
        })
    }

    /// The eight ratings, in display order.
    // The doctrine's kernel order: three primary stocks · three derived
    // exchange rates · two freedom scores from the Appendix A weights.
    pub const ALL: [SortField; 8] = [
        Self::Capital, Self::Population, Self::Territory,
        Self::Human, Self::Land, Self::Density,
        Self::Freedom, Self::Hospitality,
    ];

    /// True where a derived group begins — the pill rows draw a dot here.
    pub fn derived_break(&self) -> bool {
        matches!(self, Self::Human | Self::Freedom)
    }
}
