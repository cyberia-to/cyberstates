//! Fetches visa requirement data from passport-index-dataset on GitHub
//! and updates the TOML files in states/ with visa access arrays.
//!
//! Run: cargo run --bin fetch-visa-data --features tools
//!
//! Data source: https://github.com/ilyankou/passport-index-dataset
//! Format: Passport,Destination,Requirement
//! Requirement is either a number (visa-free days) or a string (access type)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

const CSV_URL: &str = "https://raw.githubusercontent.com/ilyankou/passport-index-dataset/master/passport-index-tidy.csv";

/// Maps passport-index country names to our TOML country names
fn name_mapping() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // passport-index name → our TOML name
    m.insert("Czech Republic", "Czechia");
    m.insert("DR Congo", "Congo");
    m.insert("Congo", "Brazzaville");
    m.insert("Ivory Coast", "Ivory Coast");
    m.insert("Cote d'Ivoire", "Ivory Coast");
    m.insert("Swaziland", "Swaziland");
    m.insert("Eswatini", "Swaziland");
    m.insert("Vatican", "Vatican");
    m.insert("Vatican City", "Vatican");
    m.insert("Macao", "Macau");
    m.insert("United Arab Emirates", "Emirates");
    m.insert("Antigua and Barbuda", "Antigua");
    m.insert("Bosnia and Herzegovina", "Bosnia");
    m.insert("Burkina Faso", "Burkina");
    m.insert("Central African Republic", "Centrafrique");
    m.insert("Dominican Republic", "Dominicana");
    m.insert("Equatorial Guinea", "Ecuatorial");
    m.insert("Guinea-Bissau", "Bissau");
    m.insert("Saint Kitts and Nevis", "Kitts");
    m.insert("South Korea", "Korea");
    m.insert("Saint Lucia", "Lucia");
    m.insert("Marshall Islands", "Marshalls");
    m.insert("North Macedonia", "Macedonia");
    m.insert("Papua New Guinea", "Papua");
    m.insert("Saudi Arabia", "Saudi");
    m.insert("Solomon Islands", "Solomons");
    m.insert("Sao Tome and Principe", "Sao Tome");
    m.insert("El Salvador", "Salvador");
    m.insert("Saint Vincent and the Grenadines", "Vincy");
    m.insert("Timor-Leste", "Timor");
    m.insert("Trinidad and Tobago", "Trinidad");
    m.insert("United States", "America");
    m.insert("United Kingdom", "Britain");
    m.insert("Democratic Republic of the Congo", "Congo");
    m
}

/// Reverse mapping: our TOML names → passport-index names
fn reverse_name_mapping() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // our TOML name → passport-index name
    m.insert("Czechia", "Czech Republic");
    m.insert("Congo", "DR Congo");
    m.insert("Brazzaville", "Congo");
    m.insert("Ivory Coast", "Ivory Coast");
    m.insert("Swaziland", "Swaziland");
    m.insert("Vatican", "Vatican");
    m.insert("Macau", "Macao");
    m.insert("Emirates", "United Arab Emirates");
    m.insert("Antigua", "Antigua and Barbuda");
    m.insert("Bosnia", "Bosnia and Herzegovina");
    m.insert("Burkina", "Burkina Faso");
    m.insert("Centrafrique", "Central African Republic");
    m.insert("Dominicana", "Dominican Republic");
    m.insert("Ecuatorial", "Equatorial Guinea");
    m.insert("Bissau", "Guinea-Bissau");
    m.insert("Kitts", "Saint Kitts and Nevis");
    m.insert("Korea", "South Korea");
    m.insert("Lucia", "Saint Lucia");
    m.insert("Marshalls", "Marshall Islands");
    m.insert("Macedonia", "North Macedonia");
    m.insert("Papua", "Papua New Guinea");
    m.insert("Saudi", "Saudi Arabia");
    m.insert("Solomons", "Solomon Islands");
    m.insert("Sao Tome", "Sao Tome and Principe");
    m.insert("Salvador", "El Salvador");
    m.insert("Vincy", "Saint Vincent and the Grenadines");
    m.insert("Timor", "Timor-Leste");
    m.insert("Trinidad", "Trinidad and Tobago");
    m.insert("America", "United States");
    m.insert("Britain", "United Kingdom");
    m
}

#[derive(Debug, Clone)]
struct VisaEntry {
    destination: String, // our TOML name
    access_type: String, // "visa-free", "visa-on-arrival", "eta", "e-visa", "visa-required", "no-admission"
    days: Option<u32>,
}

fn parse_requirement(req: &str) -> (String, Option<u32>) {
    let req = req.trim();
    if let Ok(days) = req.parse::<i32>() {
        if days == -1 {
            return ("self".to_string(), None);
        }
        return ("visa-free".to_string(), Some(days as u32));
    }
    match req {
        "visa free" => ("visa-free".to_string(), None),
        "visa on arrival" => ("visa-on-arrival".to_string(), None),
        "eta" => ("eta".to_string(), None),
        "e-visa" => ("e-visa".to_string(), None),
        "visa required" => ("visa-required".to_string(), None),
        "no admission" => ("no-admission".to_string(), None),
        _ => ("unknown".to_string(), None),
    }
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn main() {
    eprintln!("Fetching visa data from {}", CSV_URL);
    let body = reqwest::blocking::get(CSV_URL)
        .expect("Failed to fetch CSV")
        .text()
        .expect("Failed to read response body");

    let name_map = name_mapping();

    // Parse CSV into: passport_name -> Vec<VisaEntry>
    let mut data: HashMap<String, Vec<VisaEntry>> = HashMap::new();
    let mut skipped_passports: Vec<String> = Vec::new();

    for line in body.lines().skip(1) {
        // CSV format: Passport,Destination,Requirement
        // Some country names contain commas? Unlikely here but handle simple case
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() != 3 {
            continue;
        }

        let raw_passport = parts[0].trim();
        let raw_dest = parts[1].trim();
        let raw_req = parts[2].trim();

        // Map names to our TOML names
        let passport = name_map
            .get(raw_passport)
            .unwrap_or(&raw_passport)
            .to_string();
        let dest = name_map.get(raw_dest).unwrap_or(&raw_dest).to_string();

        let (access_type, days) = parse_requirement(raw_req);

        // Skip self-entries
        if access_type == "self" {
            continue;
        }

        data.entry(passport).or_default().push(VisaEntry {
            destination: dest,
            access_type,
            days,
        });
    }

    // Now read each TOML file, find matching passport data, and append visa_access
    let states_dir = Path::new("states");
    let reverse_map = reverse_name_mapping();

    let mut updated = 0;
    let mut not_found = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(states_dir)
        .expect("Cannot read states/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "toml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let content = fs::read_to_string(&path).expect("Cannot read file");

        // Extract country name from TOML
        let country_name = content
            .lines()
            .find(|l| l.starts_with("name = "))
            .and_then(|l| l.strip_prefix("name = \""))
            .and_then(|l| l.strip_suffix('"'))
            .unwrap_or("")
            .to_string();

        if country_name.is_empty() {
            eprintln!("  Warning: no name in {:?}", path);
            continue;
        }

        // Try our name first, then the reverse-mapped name
        let visa_entries = data.get(&country_name).or_else(|| {
            reverse_map
                .get(country_name.as_str())
                .and_then(|alt| data.get(*alt))
        });

        let entries = match visa_entries {
            Some(e) => e,
            None => {
                not_found.push(country_name.clone());
                continue;
            }
        };

        // Strip any existing [[visa_access]] section from the file
        let base_content: String = content
            .lines()
            .take_while(|l| !l.starts_with("[[visa_access]]"))
            .collect::<Vec<_>>()
            .join("\n");

        // Build new visa_access section
        let mut visa_section = String::new();
        visa_section.push_str("\n");

        // Sort entries: visa-free first, then by destination name
        let mut sorted_entries = entries.clone();
        sorted_entries.sort_by(|a, b| {
            let type_order = |t: &str| -> u8 {
                match t {
                    "visa-free" => 0,
                    "visa-on-arrival" => 1,
                    "eta" => 2,
                    "e-visa" => 3,
                    "visa-required" => 4,
                    "no-admission" => 5,
                    _ => 6,
                }
            };
            type_order(&a.access_type)
                .cmp(&type_order(&b.access_type))
                .then(a.destination.cmp(&b.destination))
        });

        for ve in &sorted_entries {
            visa_section.push_str("[[visa_access]]\n");
            visa_section.push_str(&format!(
                "country = \"{}\"\n",
                escape_toml_string(&ve.destination)
            ));
            visa_section.push_str(&format!("type = \"{}\"\n", ve.access_type));
            if let Some(days) = ve.days {
                visa_section.push_str(&format!("days = {}\n", days));
            }
            visa_section.push('\n');
        }

        let new_content = format!("{}\n{}", base_content.trim_end(), visa_section);
        fs::write(&path, new_content).expect("Cannot write file");
        updated += 1;

        if updated % 20 == 0 {
            eprintln!("  Updated {}/{} countries...", updated, entries.len());
        }
    }

    eprintln!("\nDone!");
    eprintln!("  Updated: {} countries", updated);
    if !not_found.is_empty() {
        eprintln!("  Not found in passport-index data ({}):", not_found.len());
        for n in &not_found {
            eprintln!("    - {}", n);
        }
    }
    if !skipped_passports.is_empty() {
        eprintln!("  Skipped passports: {:?}", skipped_passports);
    }
}
