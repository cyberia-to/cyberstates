#!/usr/bin/env python3
"""
Refresh token_price_usd and money_supply_b_usd in states/*.toml.

Sources:
- FX rates:    open.er-api.com (USD base, ~160 currencies, updated daily)
- Broad money: World Bank FM.LBL.BMNY.CN (current LCU, most recent non-empty
               year, accepted only if >= MIN_YEAR)
- US M2:       FRED M2SL via keyless fredgraph.csv (World Bank stopped
               publishing US broad money)

Per state:
- price: fresh FX where the code exists; otherwise the old price stays.
- supply: World Bank LCU x fresh price where available; otherwise the old
  USD cap is revalued by the price change (token supply preserved).

Prints a coverage report and the biggest movers. Run from repo root:
    python3 update_market_data.py
"""

import glob
import json
import re
import urllib.request

MIN_YEAR = 2018

def fetch(url):
    with urllib.request.urlopen(url, timeout=60) as r:
        return r.read().decode()

# --- FX ---------------------------------------------------------------
fx = json.loads(fetch("https://open.er-api.com/v6/latest/USD"))
assert fx["result"] == "success"
rates = fx["rates"]  # units of X per 1 USD
fx_date = fx["time_last_update_utc"]

# --- World Bank broad money (LCU) ------------------------------------
wb_raw = json.loads(fetch(
    "https://api.worldbank.org/v2/country/all/indicator/FM.LBL.BMNY.CN"
    "?format=json&mrnev=1&per_page=400"))
wb = {}
for row in wb_raw[1]:
    if row["value"] is None:
        continue
    year = int(row["date"])
    if year >= MIN_YEAR:
        wb[row["country"]["id"]] = (row["value"], year)

# --- US M2 from FRED --------------------------------------------------
us_m2_b = None
try:
    csv = fetch("https://fred.stlouisfed.org/graph/fredgraph.csv?id=M2SL")
    last = [l for l in csv.strip().split("\n")[1:] if "," in l][-1]
    us_date, us_val = last.split(",")
    us_m2_b = float(us_val)  # M2SL is in billions USD
    wb["US"] = (us_m2_b * 1e9, int(us_date[:4]))  # store as "LCU"=USD
except Exception as e:
    print(f"warn: FRED M2 fetch failed ({e}); US falls back to revalue")

# --- rewrite tomls ----------------------------------------------------
re_code = re.compile(r'^code = "([^"]+)"', re.M)
re_cur = re.compile(r'^currency_code = "([^"]+)"', re.M)
re_cap = re.compile(r'^money_supply_b_usd = ([0-9.eE+-]+)', re.M)
re_price = re.compile(r'^token_price_usd = ([0-9.eE+-]+)', re.M)

def fmt(v):
    if v >= 100: return f"{v:.0f}"
    if v >= 1:   return f"{v:.1f}"
    return f"{v:.3f}"

stats = {"wb": 0, "revalued": 0, "no_fx": [], "kept": 0}
movers = []

for path in sorted(glob.glob("states/*.toml")):
    s = open(path).read()
    code = re_code.search(s).group(1)
    cur = re_cur.search(s).group(1)
    old_cap = float(re_cap.search(s).group(1))
    old_price = float(re_price.search(s).group(1))

    if cur in rates and rates[cur] > 0:
        new_price = 1.0 / rates[cur]
    else:
        new_price = old_price
        stats["no_fx"].append(f"{code}:{cur}")

    if code in wb:
        lcu, year = wb[code]
        new_cap = lcu * new_price / 1e9
        stats["wb"] += 1
        src = f"WB{year}"
    elif old_price > 0:
        ratio = new_price / old_price
        if ratio > 3 or ratio < 1 / 3:
            # a >3x FX jump without fresh supply data means redenomination
            # or a broken old quote — revaluing units is invalid, keep cap
            new_cap = old_cap
            stats["kept"] += 1
            src = "kept:fx-jump"
        else:
            new_cap = old_cap * ratio
            stats["revalued"] += 1
            src = "reval"
    else:
        new_cap = old_cap
        stats["kept"] += 1
        src = "kept"

    if old_cap > 1:
        movers.append((abs(new_cap / old_cap - 1), code, cur, old_cap, new_cap, src))

    s = re_price.sub(f"token_price_usd = {new_price:.6f}", s)
    s = re_cap.sub(f"money_supply_b_usd = {fmt(new_cap)}", s)
    open(path, "w").write(s)

print(f"FX as of: {fx_date}")
print(f"states: WB-updated={stats['wb']}  revalued={stats['revalued']}  kept={stats['kept']}")
print(f"no FX rate (price kept): {', '.join(stats['no_fx']) or 'none'}")
print("\nbiggest cap movers:")
for d, code, cur, o, n, src in sorted(movers, reverse=True)[:15]:
    print(f"  {code} {cur}: ${o:,.0f}B -> ${n:,.0f}B  ({(n/o-1)*+100:+.0f}%, {src})")
