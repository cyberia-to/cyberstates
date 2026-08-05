# Validate state slugs in content: required, unique, [a-z0-9-]+

def main [] {
  let rows = (
    glob states/*.toml
    | each {|f|
        let r = open $f
        {code: $r.code, name: $r.name, slug: ($r.slug? | default ""), file: $f}
      }
  )
  let missing = ($rows | where {|r| $r.slug == ""})
  if not ($missing | is-empty) {
    print "MISSING slug:"
    print $missing
    error make {msg: $"($missing | length) states missing slug"}
  }
  let bad = (
    $rows
    | where {|r| not ($r.slug | str replace --all --regex '[a-z0-9-]+' '' | is-empty | default false) == false }
  )
  # simpler validity: chars only a-z0-9-
  let invalid = (
    $rows
    | where {|r|
        let cleaned = ($r.slug | str replace --all --regex '[a-z0-9-]' '')
        $cleaned != ""
      }
  )
  if not ($invalid | is-empty) {
    print "INVALID slug charset:"
    print $invalid
    error make {msg: "invalid slug charset"}
  }
  let coll = (
    $rows
    | group-by slug
    | transpose slug items
    | where {|x| ($x.items | length) > 1}
  )
  if not ($coll | is-empty) {
    print "COLLISIONS:"
    print $coll
    error make {msg: "slug collisions"}
  }
  print $"ok: ($rows | length) unique slugs"
  $rows | where code in ["US" "JP" "GB" "HK" "KR" "CD"] | print
}
