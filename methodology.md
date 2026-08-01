# Methodology

How the FREEDOM and OPENNESS scores are computed for all 229 states.

## The idea

A passport is an option contract on the planet. Its value is not how many
*countries* you can enter — counting Liechtenstein and China as equal units is
nonsense — but how much of the world's **economy** and **population** you can
reach with it. Symmetrically, a state's openness is not how many passports it
admits, but how much of the world it lets in.

Two directions, two scores:

- **FREEDOM** — what the state's passport opens *outward* for its holders.
- **OPENNESS** — what the state's border opens *inward* for everyone else.

## Access weights

The visa matrix covers every ordered pair of states. Each access regime gets a
weight — the fraction of the option's value that survives the friction:

| Regime | Weight | Reasoning |
|---|---|---|
| visa-free | 1.00 | walk in |
| visa-on-arrival | 0.80 | small fee and a queue at the border |
| eTA / e-visa | 0.50 | online form, approval risk, waiting time |
| visa-required | 0.10 | embassy interview: cost, delay, refusal risk — but a path exists |
| no admission | 0.00 | the border is a wall |

## Four sub-indices

For a state S, over every other state D in the matrix:

```
eco_out(S) = Σ  w(S→D) × cap(D)   / world_cap   × 100
pop_out(S) = Σ  w(S→D) × pop(D)   / world_pop   × 100

eco_in(S)  = Σ  w(D→S) × cap(D)   / world_cap   × 100
pop_in(S)  = Σ  w(D→S) × pop(D)   / world_pop   × 100
```

where `w(A→B)` is the access weight a holder of A's passport faces at B's
border, `pop` is population, and `cap` is the state's **money supply in USD**
— the same figure as the CAP column. Economy here means the stock of the
state's token in circulation, not annual GDP flow: a state is scored by how
much monetized wealth its passport reaches, in keeping with the token frame of
the whole table.

Each sub-index reads as "weighted percent of the world":
`eco_out = 62` means the passport reaches 62% of the world's money at full
value after visa friction.

## Folding four into two

```
FREEDOM  = √( eco_out × pop_out )
OPENNESS = √( eco_in  × pop_in  )
```

The geometric mean, not the arithmetic one, and deliberately so:

1. **Both dimensions must be real.** A passport that reaches all the money but
   none of the people (or the reverse) is a broken option. The geometric mean
   punishes imbalance: √(80 × 20) = 40, while (80+20)/2 = 50 would flatter it.
2. **Zero is contagious.** A hypothetical passport admitted nowhere scores 0,
   not "half of whatever the other axis says".
3. **Scale stays intuitive.** Both inputs are 0–100, so the score is 0–100.

## Worked example (illustrative numbers)

A passport with visa-free access to the EU and the US, e-visa to China, and
visa-required for India:

```
eco_out = 1.0×(EU cap) + 1.0×(US cap) + 0.5×(CN cap) + 0.1×(IN cap) + …
```

China's $42T money supply enters at half weight — an e-visa regime costs the
holder 50% of that option's value. India's $2.7T enters at one tenth. Summed,
normalized by world totals, folded by the geometric mean: one number.

## Data

- **Visa matrix** — scraped from public passport-index data; every ordered
  pair of the 229 states, five regimes.
- **Population, land area, money supply, token prices** — one TOML file per
  state in [`states/`](https://github.com/cyberia-to/cyberstates/tree/main/states),
  assembled from public statistics; money supply converted to USD at current
  token prices.
- Everything is baked into the app at build time — the table you see is a
  snapshot, not a live feed.

## Honest limitations

- **Money supply ≠ GDP.** The economy axis rewards monetized wealth stock.
  States with deep money (China's M2) weigh more than their GDP share; poor
  but populous states weigh in through the population axis instead.
- **Weights are judgment calls.** 0.8 for visa-on-arrival and 0.1 for
  embassy-visa are defensible, not derivable. Shifting them reshuffles the
  middle of the table, not the extremes.
- **The matrix is a snapshot.** Visa regimes change weekly; the table is as
  fresh as its last scrape.
- **Bilateral nuance is flattened.** Duration of stay, work rights, refusal
  rates, and reciprocity asymmetries are not modeled — a 30-day visa-free and
  a 180-day visa-free both count 1.0.
- **229 states include partially recognized ones.** Their matrices are
  sparser and their scores accordingly conservative.
