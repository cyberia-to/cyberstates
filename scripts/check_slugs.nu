def slugify [s: string] {
  $s | str downcase
    | str replace --all " " "-"
    | str replace --all "'" ""
    | str replace --all "." ""
    | str replace --all "," ""
    | str replace --all "/" "-"
}

def main [] {
  let rows = (glob states/*.toml | each {|f| open $f })
  let slugs = ($rows | each {|r| {code: $r.code, name: $r.name, slug: (slugify $r.name)} })
  print $"total ($slugs | length) unique ($slugs | get slug | uniq | length)"
  let coll = ($slugs | group-by slug | transpose slug items | where {|x| ($x.items | length) > 1})
  if ($coll | is-empty) {
    print "no collisions"
  } else {
    print $coll
  }
  # sample
  $slugs | where code in ["US" "JP" "GB" "HK" "CD" "CG" "KP" "KR" "AE" "NZ"] | print
}
