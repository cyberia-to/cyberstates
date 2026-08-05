# IndexNow — optional crawl notify (Bing / Yandex / compatible engines).
# Use when cutting a release, not on every routine deploy.
#
#   nu scripts/deploy.nu --release     # preferred
#   nu scripts/indexnow.nu             # after rsync, full sitemap
#   nu scripts/indexnow.nu --priority  # core + states + tokens only
#
# Key must already be live:
#   https://cyberstates.net/cyberstates-indexnow-8f3a2c1e9b7d4a60.txt
# Batches of ≤10_000. Exit non-zero if a batch fails hard.

const BASE = "https://cyberstates.net"
const KEY = "cyberstates-indexnow-8f3a2c1e9b7d4a60"
const HOST = "cyberstates.net"
const BATCH = 10000
const ENDPOINTS = [
  "https://api.indexnow.org/indexnow"
  "https://www.bing.com/indexnow"
]

def project_root [] {
  if ($"($env.PWD)/states" | path exists) { $env.PWD } else {
    error make {msg: "run from cyberstates root"}
  }
}

def extract_locs [xml_path: string] {
  if not ($xml_path | path exists) { return [] }
  open --raw $xml_path
  | lines
  | where {|l| $l | str contains "<loc>"}
  | each {|l|
      $l
      | str replace --all --regex '.*<loc>' ''
      | str replace --all --regex '</loc>.*' ''
      | str trim
    }
  | where {|u| ($u | str length) > 0}
}

# Sitemap index → child sitemaps → every <loc>
def load_urls [out_dir: string, priority_only: bool] {
  mut names = []
  if $priority_only {
    $names = ["sitemap-core.xml" "sitemap-states.xml" "sitemap-tokens.xml"]
  } else {
    let index = $"($out_dir)/sitemap.xml"
    if not ($index | path exists) {
      error make {msg: $"missing ($index) — run nu scripts/build.nu first"}
    }
    $names = (
      extract_locs $index
      | each {|u|
          $u
          | str replace $"($BASE)/" ""
        }
    )
  }
  mut urls = []
  for name in $names {
    let path = $"($out_dir)/($name)"
    let locs = (extract_locs $path)
    $urls = ($urls | append $locs)
  }
  # always include the homepage + sitemap itself
  $urls = ($urls | append [$"($BASE)/" $"($BASE)/sitemap.xml"] | uniq | sort)
  $urls
}

def post_batch [url_list: list<string>, endpoint: string] {
  let body = {
    host: $HOST
    key: $KEY
    keyLocation: $"($BASE)/($KEY).txt"
    urlList: $url_list
  } | to json

  let res = (
    http post
      --full
      --allow-errors
      --content-type "application/json; charset=utf-8"
      $endpoint
      $body
  )
  let status = ($res.status? | default 0)
  # IndexNow: 200 OK, 202 Accepted — success. 422 key mismatch, 4xx/5xx fail.
  if $status in [200 202] {
    {ok: true, status: $status, endpoint: $endpoint}
  } else {
    {ok: false, status: $status, endpoint: $endpoint, body: ($res.body? | default "" | into string | str substring 0..200)}
  }
}

def main [
  --out: string = "dist",   # directory with sitemap*.xml (post-build)
  --priority                # skip corridor sitemaps (faster smoke)
] {
  let root = (project_root)
  let out_dir = if ($out | str starts-with "/") { $out } else { $"($root)/($out)" | path expand }

  let key_file = $"($out_dir)/($KEY).txt"
  if not ($key_file | path exists) {
    error make {msg: $"IndexNow key missing at ($key_file) — prerender should write it"}
  }

  # live key check (after deploy)
  let live = try {
    http get --full --allow-errors $"($BASE)/($KEY).txt"
  } catch {|e|
    error make {msg: $"IndexNow key not reachable at ($BASE)/($KEY).txt — deploy dist/ first: ($e.msg?)"}
  }
  let live_status = ($live.status? | default 0)
  let live_body = ($live.body? | default "" | into string | str trim)
  if $live_status != 200 or $live_body != $KEY {
    error make {msg: $"IndexNow key live check failed status=($live_status) body=($live_body)"}
  }
  print $"✓ key live at ($BASE)/($KEY).txt"

  let urls = (load_urls $out_dir $priority)
  let n = ($urls | length)
  if $n == 0 {
    error make {msg: "no URLs to submit — empty sitemaps?"}
  }
  print $"IndexNow → ($n) urls  \(batch ($BATCH)\)"

  mut failed = 0
  mut batch_i = 0
  for chunk in ($urls | chunks $BATCH) {
    $batch_i = $batch_i + 1
    let size = ($chunk | length)
    mut batch_ok = false
    for ep in $ENDPOINTS {
      let r = try {
        post_batch $chunk $ep
      } catch {|e|
        {ok: false, status: 0, endpoint: $ep, body: ($e.msg? | default "error")}
      }
      if $r.ok {
        print $"  batch ($batch_i) ($size) urls → ($ep)  ($r.status)"
        $batch_ok = true
        break
      } else {
        print $"  batch ($batch_i) ($ep) failed status=($r.status) ($r.body? | default '')"
      }
    }
    if not $batch_ok {
      $failed = $failed + 1
    }
  }

  if $failed > 0 {
    error make {msg: $"IndexNow: ($failed) batch\(es\) failed"}
  }
  print $"✓ IndexNow complete — ($n) urls notified"
}
