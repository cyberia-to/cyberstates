# Doctrine

A cyberstate is a territory with a population that establishes the rules
in force on it. The rules are its protocol. The currency the rules demand
is its token. The wealth the world agrees to hold in that token is its
price.

The market for these tokens has always existed. Migration is a trade.
Capital flight is a sell-off. Citizenship is a position: most humans
alive hold at least one, and the millions who hold none have a name —
apatrides, the stateless — proof by existence that a passport is a
holding, not a birthright. What the market never had is a terminal:
no ticker, no quotes, no listing rules.

Cyberstates is that terminal: all 230 cyberstates quoted under one
doctrine, from no privileged point of view.

## The cyberstate

A row in this table is not a UN seat. It is a **cyberstate**: a territory
whose population establishes the rules in force on it — the border regime,
the token, the legal surface.

Sovereignty, in this frame, is not binary. It is a palette of rule
domains — border, token, law, defense, taxation — and each domain can be
held locally, inherited from a larger state, delegated upward to a union,
or overridden outright. Cyberstates nest, subordinate and inherit in both
directions:

- **Micronesia** (the Federated States) is one row: four member states
  inherit most rules from their federation; the federation holds border
  and law locally, rents its token from America and delegates defense to
  it by compact. The Marshall Islands and Palau share its ocean and its
  regional name — separate rows, because their rule-sets are their own.
- **Guam** inherits American sovereignty wholesale, then overrides the
  domains a visitor actually touches: its own entry waiver, its own tax
  code. A row with no UN seat.
- **Hong Kong** inherits China's sovereignty and overrides token, border
  and law — three domains that make it more distinct at street level than
  many UN members. **Greenland** does the same under Denmark.
- The **eurozone** inherits upward: thirty cyberstates delegate the token
  domain to a shared mint while keeping their borders and laws. Token and
  state are many-to-many — which is why tokens have their own table.
- **Transnistria** holds every domain de facto and none de jure.
  **Antarctica** is the limit case: a rule-set with no owner at all — a
  treaty instead of a throne — zero capital, the planet's second-largest
  territory.

Nobody interacts with sovereignty in the abstract. You land at a border,
hold a token, sign under a law. The cyberstate is the surface where rules
touch humans — and every row of this table is such a surface, whatever
pyramid of inheritance stands behind it.

## The accounting

A state issues a currency and demands its use. That currency is the state's
token, and the wealth the world agrees to hold in it is the token's
capitalization. Read the columns of any exchange against this table:

| on an exchange | here |
|---|---|
| token | the state's currency |
| price | USD per unit |
| supply | units in circulation |
| market cap | money supply — the state's CAPITAL |
| holders | population |
| protocol rules | borders, rights, visa regimes |

This is not a metaphor — it is the same accounting. A state people trust
holds more of the world's wealth in its token; a state people flee bleeds
cap. The ratings are the quotes of this market, and the pages behind them
are its asset profiles.

## Fundamental and derived

Two kinds of numbers, never confused:

**Fundamental** — measured, not computed: population, area, price, supply,
capital. They come from the world as it is.

**Derived** — computed from fundamentals, and every derived number on this
site carries its formula next to its value:

| rating | formula | what it answers |
|---|---|---|
| CAPITAL | Σ money supply, in USD | how much wealth lives in this token |
| HUMAN VALUE | capital / population | how much wealth stands behind each person |
| LAND VALUE | capital / area | how monetized each km² is |
| DENSITY | population / area | how crowded the land is |
| TRAVEL FREEDOM | √(eco_out × pop_out) | how much of the world this passport opens |
| HOSPITALITY | √(eco_in × pop_in) | how much of the world this border admits |

The taxonomy is the audit trail: challenge a derived number and the dispute
reduces to fundamentals and arithmetic — both inspectable.

## Capital: why money stock, not GDP

GDP is the nonsensical metric here, not the alternative. It is an accounting
flow — a year of transactions, inflated by government spending, imputed rents
and statistical revisions — and it answers a question nobody at a border is
asking. A visa does not grant you a share of last year's transactions. It
grants you access to a territory where wealth is **held**.

Money supply is the stock of the state's token in circulation: the market's
own ledger of how much monetized wealth lives behind that border. It is
measured the same way this table measures everything else — a state is a
token, its economy is that token's cap. Scoring states by money stock
instead of GDP flow is not a proxy or a compromise; it is the correct
measure for instruments whose value is access to wealth, not participation
in accounting.

The practical difference is visible in the table: China's deep money
(M2 ≈ ¥307T) makes it the #1 state by capital, ahead of America — a
different world order than the GDP league tables print.

## Human value and land value

The two ratios are the deepest cut in the table — the derivatives that only
exist once you accept the capital frame.

**Human value** is the capital standing behind each member of the state.
Not income, not productivity — the monetized trust the state's token
carries per person. Luxembourg backs each resident with ~$484k, Monaco
with ~$295k, Switzerland with ~$137k; at the other end of the table the
number collapses to double digits. When a person migrates, this is the
number that changes: they move their life from one token's backing to
another's.

**Land value** is the monetization density of territory. Monaco runs at
~$5.3B per km², Singapore at ~$760M; Antarctica at exactly zero — 14.2M km²
entirely outside money. Where the number is high, the state has turned
territory into ledger; where it is low, land exists that money has not yet
reached. This is the supply curve of the planet's unmonetized space.

## The freedom pair

A passport is an option contract on the planet. Its value is not how many
*countries* it enters — counting Liechtenstein and China as equal units is
nonsense — but how much of the world's **capital** and **population** it
reaches. Symmetrically, a state's hospitality is not how many passports it
admits, but how much of the world it lets in.

The visa matrix covers every ordered pair of states; each access regime
keeps a fraction of the option's value:

| Regime | Weight | Reasoning |
|---|---|---|
| visa-free | 1.00 | walk in |
| visa-on-arrival | 0.80 | small fee and a queue at the border |
| eTA / e-visa | 0.50 | online form, approval risk, waiting time |
| visa-required | 0.10 | embassy interview: cost, delay, refusal risk — but a path exists |
| no admission | 0.00 | the border is a wall |

For a state S, over every other state D:

```
eco_out(S) = Σ  w(S→D) × capital(D) / world_capital × 100
pop_out(S) = Σ  w(S→D) × pop(D)     / world_pop     × 100

eco_in(S)  = Σ  w(D→S) × capital(D) / world_capital × 100
pop_in(S)  = Σ  w(D→S) × pop(D)     / world_pop     × 100
```

Each sub-index reads as "weighted percent of the world": `eco_out = 62`
means the passport reaches 62% of the world's money at full value after
visa friction. The fold into one score:

```
TRAVEL FREEDOM = √( eco_out × pop_out )
HOSPITALITY    = √( eco_in  × pop_in  )
```

The geometric mean, deliberately:

1. **Both dimensions must be real.** A passport that reaches all the money
   but none of the people (or the reverse) is a broken option. The
   geometric mean punishes imbalance: √(80 × 20) = 40, where the arithmetic
   mean would flatter it with 50.
2. **Zero is contagious.** A passport admitted nowhere scores 0, not "half
   of whatever the other axis says".
3. **Scale stays intuitive.** Both inputs are 0–100, so the score is 0–100.

**Movement is only the first freedom.** The same shape measures every other
liberty a state grants or withholds: the freedom to hold and move capital,
to own land as a foreigner, to speak, to leave — each is an option over the
world's capital and population, each can be weighted, summed and folded
exactly like the visa matrix. The freedom stack will grow; the methodology
will not change shape.

## No privileged numeraire

The table prices in USD by default because the dollar is the world's
current unit of account — a convention, not a truth. The numeraire switch
re-denominates every capital and price into CNY, BTC, ETH, or gold, and
the convention becomes visible: America's capital is ¥157T in the #1
state's token, ₿370M in bitcoin — more bitcoin than will ever exist —
and China's is its own ¥307T, the cleanest self-reference in the table.

The dollar is a row in this table, not its ruler. Ranks are invariant to
the numeraire; intuitions are not — which is exactly why the switch exists.

## Ranks and reading

- **Every rating reads top-down.** There is no ascending mode: a rating is
  a claim about what matters, and its table starts with the most of it.
- **Every state carries its ranks.** A state page shows the position in all
  eight ratings — China: capital #1, population #2, area #4, travel
  freedom #199. A profile in eight numbers.
- **The map is the table painted.** Money and score ratings color by rank
  percentile; population and area — by log magnitude (their polygons
  already encode size); density inverts, because green must always mean
  better and red worse, whatever the axis.

## Who gets listed

230 cyberstates: UN members, their autonomous territories, partially
recognized and unrecognized states — Abkhazia, Transnistria, Somaliland,
Kosovo and their kin — and Antarctica.

The listing rule is existence, not recognition: a territory, a population,
a rule-set of its own. Recognition is one state's opinion about another;
this table records where the rules actually differ. The registry is
open-ended, and the threshold is exact: any community becomes a cyberstate
the day its rules become the interface of a territory. Nothing else was
ever required — the old states just had a head start.

## Data

- **Visa matrix** — scraped from public passport-index data; every ordered
  pair of the 229 visa-issuing states, five regimes. Antarctica is listed
  but sits outside the matrix — no passports, no visas, scores of zero.
- **Money supply** — World Bank broad money (latest year ≥ 2018) and FRED
  M2 for the US, converted at live FX; refreshed by script, guarded
  against redenominations.
- **Population, area, token prices** — one TOML file per state in
  [`states/`](https://github.com/cyberia-to/cyberstates/tree/main/states),
  assembled from public statistics.
- **Numeraires** — BTC, ETH and gold (via PAXG = 1 troy oz) at refresh
  time.
- Everything is baked into the app at build time — the table is a
  snapshot, not a live feed.

## Honest limitations

- **Money aggregates are not uniform.** States report money supply on
  slightly different definitions and dates; CAPITAL is the best single USD
  figure per state, not a synchronized global snapshot. Human value and
  land value inherit this caveat — they are exactly as good as capital is.
- **Weights are judgment calls.** 0.8 for visa-on-arrival and 0.1 for
  embassy-visa are defensible, not derivable. Shifting them reshuffles the
  middle of the table, not the extremes.
- **The matrix is a snapshot.** Visa regimes change weekly; the table is as
  fresh as its last scrape.
- **Bilateral nuance is flattened.** Duration of stay, work rights,
  refusal rates and reciprocity are not modeled — a 30-day and a 180-day
  visa-free both count 1.0.
- **Partially recognized states have sparser matrices**, and their scores
  are accordingly conservative.
