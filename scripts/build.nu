# Build SEO surface into dist/ (P0–P3 HTML + sitemaps + IndexNow key file).
#
#   nu scripts/build.nu
#   nu scripts/deploy.nu              # rsync only
#   nu scripts/deploy.nu --release    # rsync + IndexNow (cut a crawl release)

def main [] {
  let root = (
    if ($"($env.PWD)/states" | path exists) { $env.PWD }
    else { error make {msg: "run from cyberstates root"} }
  )
  cd $root

  let cargo_bin = $"($env.HOME)/.cargo/bin"
  let path = $"($cargo_bin):($env.PATH | str join ':')"

  print "→ trunk build --release"
  with-env { PATH: $path } {
    ^trunk build --release
  }

  print "→ seo surface → dist/"
  ^nu $"($root)/scripts/seo.nu" --out dist

  print "→ prerender entity HTML → dist/"
  with-env { PATH: $path } {
    ^cargo run --release --features prerender --bin prerender -- dist
  }

  print ""
  print "dist/ ready."
  print "  nu scripts/deploy.nu              # rsync"
  print "  nu scripts/deploy.nu --release    # rsync + IndexNow"
  print "  # nginx redirects: sudo nginx -t && sudo systemctl reload nginx"
}
