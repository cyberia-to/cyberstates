# Ping IndexNow (Bing/Yandex) after deploy with a sample of key URLs.
# Key file must be live at https://cyberstates.net/{key}.txt
#
#   nu scripts/indexnow.nu

const BASE = "https://cyberstates.net"
const KEY = "cyberstates-indexnow-8f3a2c1e9b7d4a60"

def main [] {
  let urls = [
    $"($BASE)/"
    $"($BASE)/state/japan"
    $"($BASE)/state/united-states"
    $"($BASE)/from/united-states/to/japan"
    $"($BASE)/token/jpy"
    $"($BASE)/tokens"
    $"($BASE)/doctrine"
    $"($BASE)/by/travel-freedom"
    $"($BASE)/sitemap.xml"
  ]
  let body = {
    host: "cyberstates.net"
    key: $KEY
    keyLocation: $"($BASE)/($KEY).txt"
    urlList: $urls
  } | to json

  print $"IndexNow → ($urls | length) urls"
  try {
    let res = (
      http post
        --full
        --allow-errors
        --content-type "application/json; charset=utf-8"
        https://api.indexnow.org/indexnow
        $body
    )
    print $"status: ($res.status? | default $res)"
  } catch {|e|
    # 200/202 with empty body is success for IndexNow
    print $"done (or check key at ($BASE)/($KEY).txt): ($e)"
  }
}
