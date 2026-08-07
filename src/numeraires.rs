//! Alternative numeraires: the table can denominate state caps and token
//! prices in USD, bitcoin, ether, or gold. Rates are baked at data-refresh
//! time by `cargo run --bin update-market-data --features tools`
//! (gold via PAXG = 1 troy oz).

pub const CNY_USD: f64 = 0.147855; // USD per 1 CNY
pub const BTC_USD: f64 = 64343.0;
pub const ETH_USD: f64 = 1901.34;
pub const XAU_USD: f64 = 4243.02; // per troy oz

pub const GRAMS_PER_OZ: f64 = 31.1034768;

#[derive(Clone, Copy, PartialEq)]
pub enum Numeraire {
    Usd,
    Cny,
    Btc,
    Eth,
    Gold,
}

impl Numeraire {
    pub const ALL: [Numeraire; 5] = [Self::Usd, Self::Cny, Self::Btc, Self::Eth, Self::Gold];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Cny => "CNY",
            Self::Btc => "BTC",
            Self::Eth => "ETH",
            Self::Gold => "GOLD",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Usd => "🇺🇸 $",
            Self::Cny => "🇨🇳 ¥",
            Self::Btc => "₿",
            Self::Eth => "Ξ",
            // a text glyph, not an emoji: emoji coins render silver-gray and
            // refuse CSS color — ◉ takes the brand gold like ₿ takes orange
            Self::Gold => "◉",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Usd => "usd",
            Self::Cny => "cny",
            Self::Btc => "btc",
            Self::Eth => "eth",
            Self::Gold => "gold",
        }
    }

    pub fn from_slug(s: &str) -> Option<Self> {
        Some(match s {
            "usd" => Self::Usd,
            "cny" => Self::Cny,
            "btc" => Self::Btc,
            "eth" => Self::Eth,
            "gold" => Self::Gold,
            _ => return None,
        })
    }

    /// Brand color for the symbol — fiat rides its flag, crypto and gold
    /// get their own hues so they read as first-class citizens.
    pub fn color(&self) -> &'static str {
        match self {
            Self::Usd | Self::Cny => "#e0e0e0",
            Self::Btc => "#f7931a",
            Self::Eth => "#627eea",
            Self::Gold => "#ffd700",
        }
    }
}

/// Scale a positive number into (value, suffix) with k/M/B/T steps.
fn scaled(v: f64) -> (f64, &'static str) {
    if v >= 1e12 {
        (v / 1e12, "T")
    } else if v >= 1e9 {
        (v / 1e9, "B")
    } else if v >= 1e6 {
        (v / 1e6, "M")
    } else if v >= 1e3 {
        (v / 1e3, "k")
    } else {
        (v, "")
    }
}

fn sig(v: f64) -> String {
    if v >= 100.0 {
        format!("{:.0}", v)
    } else if v >= 10.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

/// Format a state/token cap given in billions of USD under a numeraire.
pub fn fmt_cap(b_usd: f64, n: Numeraire) -> String {
    if b_usd <= 0.0 {
        return "N/A".to_string();
    }
    let usd = b_usd * 1e9;
    match n {
        // full k/M/B/T scaling, not bare billions — a $1.6M state must not
        // round to $0B; the table and the state card share this formatter
        Numeraire::Usd => {
            let (v, s) = scaled(usd);
            format!("${}{}", sig(v), s)
        }
        Numeraire::Cny => {
            let (v, s) = scaled(usd / CNY_USD);
            format!("¥{}{}", sig(v), s)
        }
        Numeraire::Btc => {
            let (v, s) = scaled(usd / BTC_USD);
            format!("₿{}{}", sig(v), s)
        }
        Numeraire::Eth => {
            let (v, s) = scaled(usd / ETH_USD);
            format!("Ξ{}{}", sig(v), s)
        }
        Numeraire::Gold => {
            let (v, s) = scaled(usd / XAU_USD);
            format!("{}{} oz", sig(v), s)
        }
    }
}

/// Format a plain USD amount (per person, per km²) under a numeraire.
pub fn fmt_value(usd: f64, n: Numeraire) -> String {
    if usd <= 0.0 {
        return "—".to_string();
    }
    match n {
        Numeraire::Usd => {
            let (v, s) = scaled(usd);
            format!("${}{}", sig(v), s)
        }
        Numeraire::Cny => {
            let (v, s) = scaled(usd / CNY_USD);
            format!("¥{}{}", sig(v), s)
        }
        Numeraire::Btc => {
            let (v, s) = scaled(usd / BTC_USD);
            format!("₿{}{}", sig(v), s)
        }
        Numeraire::Eth => {
            let (v, s) = scaled(usd / ETH_USD);
            format!("Ξ{}{}", sig(v), s)
        }
        Numeraire::Gold => {
            let (v, s) = scaled(usd / XAU_USD);
            format!("{}{} oz", sig(v), s)
        }
    }
}

/// Split a price into (bright head, dim fraction, unit): the head carries
/// the readable magnitude, the fraction carries precision at reduced
/// size/brightness, and the unit is always visible — a number without its
/// unit is a riddle, not a price.
pub fn price_parts(usd: f64, n: Numeraire) -> (String, String, String) {
    if usd <= 0.0 {
        return ("N/A".to_string(), String::new(), String::new());
    }
    let split2 = |v: f64| {
        let full = format!("{:.2}", v);
        let (int, frac) = full.split_once('.').unwrap();
        (int.to_string(), format!(".{}", frac))
    };
    match n {
        Numeraire::Usd | Numeraire::Cny => {
            let (sym, v) = match n {
                Numeraire::Cny => ("¥", usd / CNY_USD),
                _ => ("$", usd),
            };
            let full = format!("{:.6}", v);
            let (int, frac) = full.split_once('.').unwrap();
            (
                format!("{}{}.{}", sym, int, &frac[..2]),
                frac[2..].to_string(),
                String::new(),
            )
        }
        Numeraire::Btc => {
            let (int, frac) = split2(usd / BTC_USD * 1e8);
            (int, frac, " sat".to_string())
        }
        Numeraire::Eth => {
            let (int, frac) = split2(usd / ETH_USD * 1e6);
            (int, frac, " μΞ".to_string())
        }
        Numeraire::Gold => {
            let (int, frac) = split2(usd / XAU_USD * GRAMS_PER_OZ * 1000.0);
            (int, frac, " mg Au".to_string())
        }
    }
}

/// The chosen measure survives reloads: read at boot, written on change.
pub fn load_numeraire() -> Numeraire {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item("numeraire").ok().flatten())
        .and_then(|v| Numeraire::from_slug(&v))
        .unwrap_or(Numeraire::Btc)
}

pub fn store_numeraire(n: Numeraire) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item("numeraire", n.slug());
    }
}
