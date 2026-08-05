# Rewrite /state/{code} → /state/{slug} in doctrine.md
# Reads slug from states/*.toml content. Longest codes first.

def main [] {
  let map = (
    glob states/*.toml
    | each {|f|
        let r = open $f
        if ($r.slug? | default "") == "" {
          error make {msg: $"($r.code) missing slug"}
        }
        {
          code: ($r.code | str downcase)
          slug: $r.slug
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
      let pat = ("/state/" + $row.code + '(?![a-z0-9-])')
      $text = ($text | str replace --all --regex $pat $to)
    }
  }
  $text | save --force doctrine.md
  print "doctrine.md state links → content slugs"
}
