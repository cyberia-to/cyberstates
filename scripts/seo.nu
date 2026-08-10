# Generate robots.txt + sitemaps for cyberstates.net
#
# Usage:
#   nu scripts/seo.nu              # writes to seo/
#   nu scripts/seo.nu --out dist   # after trunk build --release
#
# lastmod comes from state file mtimes (content freshness).

const BASE = "https://cyberstates.net"
const AGGREGATES = ["OCNA", "AFRI", "EURA", "AMER"]
const SKIP_REGIONS = ["Oceans", "Terra Nullius", "Solar System"]
const CHUNK = 45000

const FIELDS = [
  "capital"
  "growth-pace"
  "death-pace"
  "capital-gain"
  "capital-loss"
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

def file_lastmod [path: string] {
  let m = (ls $path | get modified | first)
  $m | format date "%Y-%m-%d"
}

def main [--out: string = "seo"] {
  let root = (project_root)
  let out_dir = if ($out | str starts-with "/") or ($out | str starts-with "~") {
    $out | path expand
  } else {
    $"($root)/($out)" | path expand
  }
  mkdir $out_dir

  let today = (date now | format date "%Y-%m-%d")
  let state_files = (glob $"($root)/states/*.toml")
  let states = (
    $state_files
    | each {|f|
        let r = open $f
        let lm = (file_lastmod $f)
        $r | insert lastmod $lm | insert file $f
      }
  )

  for r in $states {
    if ($r.slug? | default "") == "" {
      error make {msg: $"state ($r.code) missing slug — set slug in states/*.toml"}
    }
  }

  let global_lastmod = (
    $states
    | get lastmod
    | sort
    | last
  )

  let slugs_all = ($states | select slug lastmod | sort-by slug)
  let corridor_rows = (
    $states
    | where {|r| ($r.code not-in $AGGREGATES) and ($r.region not-in $SKIP_REGIONS)}
    | select slug lastmod
    | sort-by slug
  )
  # free-floating network tokens (tokens/*.toml) + state-derived currencies
  let state_tokens = (
    $states
    | get currency_code
    | where {|c| ($c | str length) > 0}
    | each {|c| $c | str downcase}
  )
  let free_tokens = (
    if ($"($root)/tokens" | path exists) {
      glob $"($root)/tokens/*.toml"
      | each {|f|
          let r = open $f
          $r.code | str downcase
        }
    } else {
      []
    }
  )
  let tokens = (
    $state_tokens
    | append $free_tokens
    | uniq
    | sort
  )
  let regions = ($states | get region | uniq | sort)

  mut core = []
  $core = ($core | append (url_entry "/" "1.0" "daily" $global_lastmod))
  for p in ["/tokens" "/totals" "/doctrine" "/methodology" "/listing" "/solar" "/map" "/cyberia"] {
    $core = ($core | append (url_entry $p "0.7" "weekly" $global_lastmod))
  }
  for f in $FIELDS {
    if $f != "capital" {
      $core = ($core | append (url_entry $"/by/($f)" "0.8" "daily" $global_lastmod))
    }
  }
  for r in $regions {
    let rs = (region_slug $r)
    $core = ($core | append (url_entry $"/in/($rs)" "0.7" "daily" $global_lastmod))
    for f in $FIELDS {
      if $f != "capital" {
        $core = ($core | append (url_entry $"/in/($rs)/by/($f)" "0.6" "weekly" $global_lastmod))
      }
    }
  }
  write_urlset $"($out_dir)/sitemap-core.xml" $core

  let state_urls = (
    $slugs_all
    | each {|row| url_entry $"/state/($row.slug)" "0.9" "weekly" $row.lastmod}
  )
  write_urlset $"($out_dir)/sitemap-states.xml" $state_urls

  let token_urls = (
    $tokens
    | each {|c| url_entry $"/token/($c)" "0.8" "weekly" $global_lastmod}
  )
  write_urlset $"($out_dir)/sitemap-tokens.xml" $token_urls

  # corridors: lastmod = max(from,to) state file dates
  let corridor_entries = (
    $corridor_rows
    | each {|a|
        $corridor_rows
        | where {|b| $b.slug != $a.slug}
        | each {|b|
            let lm = if $a.lastmod > $b.lastmod { $a.lastmod } else { $b.lastmod }
            url_entry $"/from/($a.slug)/to/($b.slug)" "0.5" "monthly" $lm
          }
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

  let index_body = (
    $sitemap_names
    | each {|name|
        let loc = (xml_escape $"($BASE)/($name)")
        [
          "  <sitemap>"
          $"    <loc>($loc)</loc>"
          $"    <lastmod>($global_lastmod)</lastmod>"
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
  print $"  states:     ($slugs_all | length)  lastmod max ($global_lastmod)"
  print $"  tokens:     ($tokens | length)"
  print $"  core urls:  ($core | length)"
  print $"  corridors:  ($n_corridors)"
  let total = (($core | length) + ($slugs_all | length) + ($tokens | length) + $n_corridors)
  print $"  total urls: ($total)"
}
