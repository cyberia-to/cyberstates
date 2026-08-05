use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct CountryToml {
    name: String,
    code: String,
    /// Canonical URL segment — authored in content, not invented here.
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
    visa_access: Vec<VisaAccessToml>,
}

#[derive(Deserialize, Serialize)]
struct VisaAccessToml {
    country: String,
    #[serde(rename = "type")]
    access_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    days: Option<u32>,
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
    println!("cargo:rerun-if-changed=states");

    let states_dir = Path::new("states");
    if !states_dir.exists() {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        fs::write(
            Path::new(&out_dir).join("countries.rs"),
            "pub fn load_countries() -> Vec<crate::data::Country> { vec![] }\n\
             pub const VISA_DATA_JSON: &str = \"{}\";\n",
        )
        .unwrap();
        return;
    }

    let mut entries: Vec<_> = fs::read_dir(states_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut code =
        String::from("pub fn load_countries() -> Vec<crate::data::Country> {\n    vec![\n");

    // We need to keep parsed data alive for references
    let mut parsed: Vec<(String, CountryToml)> = Vec::new();

    for entry in &entries {
        let content = fs::read_to_string(entry.path()).unwrap();
        let c: CountryToml = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: failed to parse {:?}: {}", entry.path(), e);
                continue;
            }
        };
        let code_key = c.code.clone();
        parsed.push((code_key, c));
    }

    // Build visa JSON map: code -> [{ country, access_type, days }]
    let mut visa_json_map: HashMap<String, &Vec<VisaAccessToml>> = HashMap::new();
    let mut seen_slugs: HashMap<String, String> = HashMap::new();

    for (code_key, c) in &parsed {
        if !c.visa_access.is_empty() {
            visa_json_map.insert(code_key.clone(), &c.visa_access);
        }

        let slug = c.slug.trim().to_string();
        if slug.is_empty() {
            panic!(
                "empty slug for state {} — set slug = \"...\" in states/*.toml",
                c.code
            );
        }
        if !slug
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        {
            panic!(
                "invalid slug {:?} for state {} — use [a-z0-9-]+ only",
                slug, c.code
            );
        }
        if let Some(other) = seen_slugs.insert(slug.clone(), c.code.clone()) {
            panic!(
                "slug collision {:?} for codes {} and {}",
                slug, other, c.code
            );
        }

        code.push_str(&format!(
            "        crate::data::Country {{\n\
             \x20           name: \"{}\".to_string(),\n\
             \x20           code: \"{}\".to_string(),\n\
             \x20           slug: \"{}\".to_string(),\n\
             \x20           flag: \"{}\".to_string(),\n\
             \x20           region: \"{}\".to_string(),\n\
             \x20           population: {},\n\
             \x20           land_area_km2: {},\n\
             \x20           currency_code: \"{}\".to_string(),\n\
             \x20           currency_name: \"{}\".to_string(),\n\
             \x20           money_supply_b_usd: {:.1},\n\
             \x20           token_price_usd: {:.6}_f64,\n\
             \x20           visa_free_destinations: {},\n\
             \x20           visa_free_inbound: {},\n\
             \x20           oceans: \"{}\".to_string(),\n\
             \x20       }},\n",
            escape(&c.name),
            escape(&c.code),
            escape(&slug),
            escape(&c.flag),
            escape(&c.region),
            c.population,
            c.land_area_km2,
            escape(&c.currency_code),
            escape(&c.currency_name),
            c.money_supply_b_usd,
            c.token_price_usd,
            c.visa_free_destinations,
            c.visa_free_inbound,
            escape(&c.oceans),
        ));
    }

    code.push_str("    ]\n}\n\n");

    // Serialize visa data as a single JSON string constant
    let visa_json = serde_json::to_string(&visa_json_map).unwrap_or_else(|_| "{}".to_string());
    let escaped_json = visa_json.replace('\\', "\\\\").replace('"', "\\\"");
    code.push_str(&format!(
        "pub const VISA_DATA_JSON: &str = \"{}\";\n",
        escaped_json
    ));

    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("countries.rs"), code).unwrap();
}
