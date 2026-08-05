# Generate robots.txt + sitemaps for cyberstates.net
#
# Usage:
#   nu scripts/seo.nu              # writes to seo/
#   nu scripts/seo.nu --out dist   # after trunk build --release
#
# URL surface:
#   core landings, /state/*, /token/*, /from/{a}/to/{b} for every
#   terrestrial non-aggregate ordered pair (long-tail corridors).
#   Self-pairs (from = to) are omitted.

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

def region_slug [r: string] {
  $r | str downcase | str replace --all " " "-"
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
  let states = ($state_files | each {|f| open $f })

  let codes_all = ($states | get code | each {|c| $c | str downcase} | sort)
  let corridor_codes = (
    $states
    | where {|r| ($r.code not-in $AGGREGATES) and ($r.region not-in $SKIP_REGIONS)}
    | get code
    | each {|c| $c | str downcase}
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

  # --- states ---
  let state_urls = (
    $codes_all
    | each {|c| url_entry $"/state/($c)" "0.9" "weekly" $lastmod}
  )
  write_urlset $"($out_dir)/sitemap-states.xml" $state_urls

  # --- tokens ---
  let token_urls = (
    $tokens
    | each {|c| url_entry $"/token/($c)" "0.8" "weekly" $lastmod}
  )
  write_urlset $"($out_dir)/sitemap-tokens.xml" $token_urls

  # --- bilateral corridors: /from/{a}/to/{b}, a != b ---
  # build as a flat list via nested each (faster than mut append in a loop)
  let corridor_entries = (
    $corridor_codes
    | each {|a|
        $corridor_codes
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
  print $"  states:     ($codes_all | length)"
  print $"  tokens:     ($tokens | length)"
  print $"  core urls:  ($core | length)"
  print $"  corridors:  ($n_corridors)  \(($corridor_codes | length) terrestrial x n-1)"
  let total = (($core | length) + ($codes_all | length) + ($tokens | length) + $n_corridors)
  print $"  total urls: ($total)"
}
