# Rewrite /state/{code} → /state/{slug} in doctrine.md
# Longest codes first so /state/sol is not eaten by /state/so.

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

def slugify [name: string] {
  $name | str downcase | str replace --all --regex "[^a-z0-9]+" "-" | str trim --char "-"
}

def state_slug [code: string, name: string] {
  let o = ($SLUG_OVERRIDES | get --optional $code)
  if $o != null { $o } else { slugify $name }
}

def main [] {
  # longest code first — prevents /state/so eating /state/sol
  let map = (
    glob states/*.toml
    | each {|f|
        let r = open $f
        {
          code: ($r.code | str downcase)
          slug: (state_slug $r.code $r.name)
        }
      }
    | sort-by {|row| $row.code | str length}
    | reverse
  )
  mut text = (open --raw doctrine.md)
  for row in $map {
    let from = $"/state/($row.code)"
    let to = $"/state/($row.slug)"
    if $from != $to {
      # only exact path segments (not prefixes of longer codes/slugs)
      let pat = ("/state/" + $row.code + '(?![a-z0-9-])')
      $text = ($text | str replace --all --regex $pat $to)
    }
  }
  $text | save --force doctrine.md
  print "doctrine.md state links → name slugs (longest-first + boundary)"
}
