# Generate robots.txt + sitemaps for cyberstates.net
#
# Usage:
#   nu scripts/seo.nu              # writes to seo/
#   nu scripts/seo.nu --out dist   # after trunk build --release
#
# URL surface:
#   core landings, /state/{name-slug}, /token/{ticker},
#   /from/{slug}/to/{slug} for every terrestrial non-aggregate pair.
#   Self-pairs omitted. Slugs match build.rs (name + SEO overrides).

const BASE = "https://cyberstates.net"
const AGGREGATES = ["OCNA", "AFRI", "EURA", "AMER"]
const SKIP_REGIONS = ["Oceans", "Terra Nullius", "Solar System"]
# keep each urlset under Google's 50_000 limit
const CHUNK = 45000

const FIELDS = [
  "capital"
  "citizen-value"
  "land-value"
  "travel-freedom"
  "hospitality"
  "population"
  "territory"
  "density"
]

# Must stay in sync with build.rs slug_override + slugify
const SLUG_OVERRIDES = {
  US: "united-states"
  GB: "united-kingdom"
  AE: "united-arab-emirates"
  KR: "south-korea"
  KP: "north-korea"
  CD: "dr-congo"
  CG: "congo-brazzaville"
  DO: "dominican-republic"
  CI: "ivory-coast"
  MK: "north-macedonia"
  CZ: "czechia"
  SZ: "eswatini"
  VA: "vatican"
  FM: "micronesia"
  MH: "marshall-islands"
  SB: "solomon-islands"
  ST: "sao-tome"
  VC: "saint-vincent"
  KN: "saint-kitts"
  LC: "saint-lucia"
  GW: "guinea-bissau"
  GQ: "equatorial-guinea"
  CF: "central-african-republic"
  BA: "bosnia-and-herzegovina"
  TT: "trinidad-and-tobago"
  AG: "antigua-and-barbuda"
  PG: "papua-new-guinea"
  NC: "new-caledonia"
  CV: "cape-verde"
  BF: "burkina-faso"
  SA: "saudi-arabia"
  ZA: "south-africa"
  SS: "south-sudan"
  NCY: "northern-cyprus"
  OST: "south-ossetia"
  BTWL: "bir-tawil"
  PMR: "transnistria"
}

def region_slug [r: string] {
  $r | str downcase | str replace --all " " "-"
}

def slugify [name: string] {
  $name
  | str downcase
  | str replace --all --regex "[^a-z0-9]+" "-"
  | str trim --char "-"
}

def state_slug [code: string, name: string] {
  let o = ($SLUG_OVERRIDES | get --optional $code)
  if $o != null { $o } else { slugify $name }
}

def xml_escape [s: string] {
  $s
  | str replace --all "&" "&amp;"
  | str replace --all "<" "&lt;"
  | str replace --all ">" "&gt;"
  | str replace --all "\"" "&quot;"
  | str replace --all "'" "&apos;"
}

def url_entry [path: string, priority: string, changefreq: string, lastmod: string] {
  let loc = (xml_escape $"($BASE)($path)")
  [
    "  <url>"
    $"    <loc>($loc)</loc>"
    $"    <lastmod>($lastmod)</lastmod>"
    $"    <changefreq>($changefreq)</changefreq>"
    $"    <priority>($priority)</priority>"
    "  </url>"
  ] | str join "\n"
}

def write_urlset [path: string, entries: list<string>] {
  let body = ($entries | str join "\n")
  let xml = (
    [
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
      "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"
      $body
      "</urlset>"
      ""
    ] | str join "\n"
  )
  $xml | save --force $path
  print $"  wrote ($path) — ($entries | length) urls"
}

def project_root [] {
  let here = $env.PWD
  if ($"($here)/states" | path exists) {
    $here
  } else if ($"($here)/../states" | path exists) {
    $"($here)/.." | path expand
  } else {
    error make {msg: "run from cyberstates root (states/ not found)"}
  }
}

def main [--out: string = "seo"] {
  let root = (project_root)
  let out_dir = if ($out | str starts-with "/") or ($out | str starts-with "~") {
    $out | path expand
  } else {
    $"($root)/($out)" | path expand
  }
  mkdir $out_dir

  let lastmod = (date now | format date "%Y-%m-%d")
  let state_files = (glob $"($root)/states/*.toml")
  let states = (
    $state_files
    | each {|f|
        let r = open $f
        $r | insert slug (state_slug $r.code $r.name)
      }
  )

  let slugs_all = ($states | get slug | sort)
  let corridor_slugs = (
    $states
    | where {|r| ($r.code not-in $AGGREGATES) and ($r.region not-in $SKIP_REGIONS)}
    | get slug
    | sort
  )
  let tokens = (
    $states
    | get currency_code
    | where {|c| ($c | str length) > 0}
    | each {|c| $c | str downcase}
    | uniq
    | sort
  )
  let regions = ($states | get region | uniq | sort)

  # --- core landings ---
  mut core = []
  $core = ($core | append (url_entry "/" "1.0" "daily" $lastmod))
  for p in ["/tokens" "/doctrine" "/methodology" "/listing" "/solar" "/map"] {
    $core = ($core | append (url_entry $p "0.7" "weekly" $lastmod))
  }
  for f in $FIELDS {
    if $f != "capital" {
      $core = ($core | append (url_entry $"/by/($f)" "0.8" "daily" $lastmod))
    }
  }
  for r in $regions {
    let rs = (region_slug $r)
    $core = ($core | append (url_entry $"/in/($rs)" "0.7" "daily" $lastmod))
    for f in $FIELDS {
      if $f != "capital" {
        $core = ($core | append (url_entry $"/in/($rs)/by/($f)" "0.6" "weekly" $lastmod))
      }
    }
  }
  write_urlset $"($out_dir)/sitemap-core.xml" $core

  # --- states (name slugs) ---
  let state_urls = (
    $slugs_all
    | each {|s| url_entry $"/state/($s)" "0.9" "weekly" $lastmod}
  )
  write_urlset $"($out_dir)/sitemap-states.xml" $state_urls

  # --- tokens (tickers stay short) ---
  let token_urls = (
    $tokens
    | each {|c| url_entry $"/token/($c)" "0.8" "weekly" $lastmod}
  )
  write_urlset $"($out_dir)/sitemap-tokens.xml" $token_urls

  # --- bilateral corridors: /from/{slug}/to/{slug}, a != b ---
  let corridor_entries = (
    $corridor_slugs
    | each {|a|
        $corridor_slugs
        | where {|b| $b != $a}
        | each {|b| url_entry $"/from/($a)/to/($b)" "0.5" "monthly" $lastmod}
      }
    | flatten
  )
  let n_corridors = ($corridor_entries | length)
  mut sitemap_names = ["sitemap-core.xml" "sitemap-states.xml" "sitemap-tokens.xml"]
  mut chunk_i = 0
  for chunk in ($corridor_entries | chunks $CHUNK) {
    let name = $"sitemap-corridors-($chunk_i).xml"
    write_urlset $"($out_dir)/($name)" $chunk
    $sitemap_names = ($sitemap_names | append $name)
    $chunk_i = $chunk_i + 1
  }

  # --- sitemap index ---
  let index_body = (
    $sitemap_names
    | each {|name|
        let loc = (xml_escape $"($BASE)/($name)")
        [
          "  <sitemap>"
          $"    <loc>($loc)</loc>"
          $"    <lastmod>($lastmod)</lastmod>"
          "  </sitemap>"
        ] | str join "\n"
      }
    | str join "\n"
  )
  (
    [
      "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
      "<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"
      $index_body
      "</sitemapindex>"
      ""
    ] | str join "\n"
  ) | save --force $"($out_dir)/sitemap.xml"
  print $"  wrote ($out_dir)/sitemap.xml — index of ($sitemap_names | length) sitemaps"

  # --- robots.txt ---
  (
    [
      "# cyberstates.net — the sovereignty terminal"
      "User-agent: *"
      "Allow: /"
      ""
      "# path landings are the indexable surface; query-string views are client filters"
      "Disallow: /*?*"
      ""
      $"Sitemap: ($BASE)/sitemap.xml"
      ""
    ] | str join "\n"
  ) | save --force $"($out_dir)/robots.txt"
  print $"  wrote ($out_dir)/robots.txt"

  print ""
  print $"seo surface → ($out_dir)"
  print $"  states:     ($slugs_all | length)"
  print $"  tokens:     ($tokens | length)"
  print $"  core urls:  ($core | length)"
  print $"  corridors:  ($n_corridors)  \(($corridor_slugs | length) terrestrial x n-1)"
  let total = (($core | length) + ($slugs_all | length) + ($tokens | length) + $n_corridors)
  print $"  total urls: ($total)"
  # smoke: famous corridors must be name-slugs
  print $"  sample:     /state/japan  /from/united-states/to/japan"
}
