//! Alternative numeraires: the table can denominate state caps and token
//! prices in USD, bitcoin, ether, or gold. Rates are baked at data-refresh
//! time by update_market_data.py (gold via PAXG = 1 troy oz).

pub const BTC_USD: f64 = 63030.0;
pub const ETH_USD: f64 = 1866.81;
pub const XAU_USD: f64 = 4043.92; // per troy oz
pub const NUMERAIRE_DATE: &str = "2026-08-01";

pub const GRAMS_PER_OZ: f64 = 31.1034768;

#[derive(Clone, Copy, PartialEq)]
pub enum Numeraire {
    Usd,
    Btc,
    Eth,
    Gold,
}

impl Numeraire {
    pub const ALL: [Numeraire; 4] = [Self::Usd, Self::Btc, Self::Eth, Self::Gold];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Usd => "$",
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

/// Format a unit price given in USD under a numeraire.
/// BTC prices in sats, ETH in micro-ether, gold in milligrams.
pub fn fmt_price(usd: f64, n: Numeraire) -> String {
    if usd <= 0.0 {
        return "N/A".to_string();
    }
    match n {
        Numeraire::Usd => {
            if usd >= 1.0 { format!("${:.2}", usd) }
            else if usd >= 0.01 { format!("${:.4}", usd) }
            else { format!("${:.6}", usd) }
        }
        Numeraire::Btc => {
            let sats = usd / BTC_USD * 1e8;
            if sats >= 100.0 { format!("{:.0} sat", sats) }
            else if sats >= 1.0 { format!("{:.1} sat", sats) }
            else { format!("{:.3} sat", sats) }
        }
        Numeraire::Eth => {
            let micro = usd / ETH_USD * 1e6;
            let (v, s) = scaled(micro);
            format!("{}{} μΞ", sig(v), s)
        }
        Numeraire::Gold => {
            let mg = usd / XAU_USD * GRAMS_PER_OZ * 1000.0;
            if mg >= 1000.0 { format!("{:.1} g Au", mg / 1000.0) }
            else if mg >= 1.0 { format!("{:.1} mg Au", mg) }
            else { format!("{:.3} mg Au", mg) }
        }
    }
}
