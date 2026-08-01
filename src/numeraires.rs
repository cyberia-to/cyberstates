//! Alternative numeraires: the table can denominate state caps and token
//! prices in USD, bitcoin, ether, or gold. Rates are baked at data-refresh
//! time by update_market_data.py (gold via PAXG = 1 troy oz).

pub const CNY_USD: f64 = 0.147822; // USD per 1 CNY
pub const BTC_USD: f64 = 63030.0;
pub const ETH_USD: f64 = 1866.81;
pub const XAU_USD: f64 = 4043.92; // per troy oz

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
            Self::Gold => "Au",
        }
    }
}

/// Scale a positive number into (value, suffix) with k/M/B/T steps.
fn scaled(v: f64) -> (f64, &'static str) {
    if v >= 1e12 { (v / 1e12, "T") }
    else if v >= 1e9 { (v / 1e9, "B") }
    else if v >= 1e6 { (v / 1e6, "M") }
    else if v >= 1e3 { (v / 1e3, "k") }
    else { (v, "") }
}

fn sig(v: f64) -> String {
    if v >= 100.0 { format!("{:.0}", v) }
    else if v >= 10.0 { format!("{:.1}", v) }
    else { format!("{:.2}", v) }
}

/// Format a state/token cap given in billions of USD under a numeraire.
pub fn fmt_cap(b_usd: f64, n: Numeraire) -> String {
    if b_usd <= 0.0 {
        return "N/A".to_string();
    }
    let usd = b_usd * 1e9;
    match n {
        Numeraire::Usd => {
            if b_usd >= 1000.0 { format!("${:.1}T", b_usd / 1000.0) }
            else { format!("${:.0}B", b_usd) }
        }
        Numeraire::Cny => {
            let cny_b = b_usd / CNY_USD;
            if cny_b >= 1000.0 { format!("¥{:.1}T", cny_b / 1000.0) }
            else { format!("¥{:.0}B", cny_b) }
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
        Numeraire::Usd => { let (v, s) = scaled(usd); format!("${}{}", sig(v), s) }
        Numeraire::Cny => { let (v, s) = scaled(usd / CNY_USD); format!("¥{}{}", sig(v), s) }
        Numeraire::Btc => { let (v, s) = scaled(usd / BTC_USD); format!("₿{}{}", sig(v), s) }
        Numeraire::Eth => { let (v, s) = scaled(usd / ETH_USD); format!("Ξ{}{}", sig(v), s) }
        Numeraire::Gold => { let (v, s) = scaled(usd / XAU_USD); format!("{}{} oz", sig(v), s) }
    }
}

/// Split a price into (bright head, dim tail) for rendering: the head
/// carries the readable magnitude, the tail carries precision digits or
/// the unit suffix at reduced size/brightness.
pub fn price_parts(usd: f64, n: Numeraire) -> (String, String) {
    if usd <= 0.0 {
        return ("N/A".to_string(), String::new());
    }
    match n {
        Numeraire::Usd | Numeraire::Cny => {
            let (sym, v) = match n {
                Numeraire::Cny => ("¥", usd / CNY_USD),
                _ => ("$", usd),
            };
            let full = format!("{:.6}", v);
            let (int, frac) = full.split_once('.').unwrap();
            (format!("{}{}.{}", sym, int, &frac[..2]), frac[2..].to_string())
        }
        Numeraire::Btc => (format!("{:.2}", usd / BTC_USD * 1e8), " sat".to_string()),
        Numeraire::Eth => (format!("{:.2}", usd / ETH_USD * 1e6), " μΞ".to_string()),
        Numeraire::Gold => (
            format!("{:.2}", usd / XAU_USD * GRAMS_PER_OZ * 1000.0),
            " mg Au".to_string(),
        ),
    }
}

