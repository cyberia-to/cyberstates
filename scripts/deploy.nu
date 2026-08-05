# Deploy dist/ to production. IndexNow only on explicit release cuts.
#
#   nu scripts/deploy.nu                 # build + rsync
#   nu scripts/deploy.nu --release       # + full IndexNow (~53k urls)
#   nu scripts/deploy.nu --release --priority-now  # IndexNow core/states/tokens only
#
# Options:
#   --skip-build     use existing dist/
#   --release        ping IndexNow after rsync (cut a release for crawlers)
#   --priority-now   with --release: skip corridor shards (faster smoke)

def main [
  --skip-build
  --release
  --priority-now
] {
  let root = (
    if ($"($env.PWD)/states" | path exists) { $env.PWD }
    else { error make {msg: "run from cyberstates root"} }
  )
  cd $root

  if not $skip_build {
    print "→ build"
    ^nu $"($root)/scripts/build.nu"
  }

  if not ($"($root)/dist/index.html" | path exists) {
    error make {msg: "dist/index.html missing — build first"}
  }

  print "→ rsync dist/ → cyberproxy:/var/www/html/cyberstates/"
  ^rsync -az --delete $"($root)/dist/" "cyberproxy:/var/www/html/cyberstates/"

  if $release {
    print "→ IndexNow (release cut)"
    if $priority_now {
      ^nu $"($root)/scripts/indexnow.nu" --out dist --priority
    } else {
      ^nu $"($root)/scripts/indexnow.nu" --out dist
    }
    print ""
    print "✓ deployed + IndexNow"
  } else {
    print ""
    print "✓ deployed (IndexNow skipped — pass --release to notify crawlers)"
  }
  print "  if nginx-redirects.conf changed: copy + sudo nginx -t && sudo systemctl reload nginx"
}
