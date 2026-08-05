# Ensure every states/*.toml has a `slug` field.
#
# Source of truth is content. This helper only fills *missing* slugs from
# the display name (slugify). It never overwrites an authored slug —
# edit the toml to change a URL.
#
#   nu scripts/write_slugs.nu
#   nu scripts/write_slugs.nu --dry-run

def slugify [name: string] {
  $name | str downcase | str replace --all --regex "[^a-z0-9]+" "-" | str trim --char "-"
}

def main [--dry-run] {
  let files = (glob states/*.toml | sort)
  mut seen = {}
  mut filled = 0

  for f in $files {
    let r = open $f
    let existing = ($r.slug? | default "")
    let slug = if $existing != "" { $existing } else { slugify $r.name }

    if $slug == "" {
      error make {msg: $"empty slug for ($f) — set slug in the toml"}
    }

    let prior = ($seen | get --optional $slug)
    if $prior != null {
      error make {msg: $"slug collision ($slug): ($prior) vs ($r.code)"}
    }
    $seen = ($seen | insert $slug $r.code)

    if $existing != "" {
      continue
    }

    # missing — insert after code =
    let raw = (open --raw $f)
    let line = $"slug = \"($slug)\""
    if not ($raw | str contains $"code = \"($r.code)\"") {
      error make {msg: $"cannot place slug in ($f) — no code line"}
    }
    let next = ($raw | str replace $"code = \"($r.code)\"" $"code = \"($r.code)\"\n($line)")

    if $dry_run {
      print $"FILL ($r.code)\t($slug)\t($f)"
    } else {
      $next | save --force $f
      $filled = $filled + 1
    }
  }

  if $dry_run {
    print $"dry-run done — ($seen | columns | length) unique slugs"
  } else {
    print $"filled ($filled) missing slugs; ($seen | columns | length) total unique"
  }
}
