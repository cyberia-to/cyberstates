# Release build + SEO surface into dist/
#
#   nu scripts/build.nu
#   rsync -az --delete dist/ cyberproxy:/var/www/html/cyberstates/

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
  print "dist/ ready. deploy:"
  print "  rsync -az --delete dist/ cyberproxy:/var/www/html/cyberstates/"
}
