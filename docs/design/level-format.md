# Level format

A level is one line of URL-safe text. `crates/cl-level` parses and serialises it; `Level::parse` then
`to_string` is the identity for every valid level (proptest).

```
CL1;n=8;seed=1234;players=3;human=R;ai=Y:h60,i2;ai=G:h30,i1;land=42;forest=10;victory=majority:60,3;tiles=12LTRp,13LFR,14LCY,40S,77-90L
```

## Grammar

```
level    = "CL1" { ";" field } ;
field    = "name=" name | "n=" int | "seed=" int | "players=" int | "human=" factions
         | "ai=" ai | "land=" int | "forest=" int | "victory=" victory | "tiles=" tiles ;
name     = 1*( ALPHA | DIGIT | "_" | "." | "-" ) ;
factions = 1*( "R" | "Y" | "G" | "B" ) ;
ai       = faction ":" [ "h" int ] [ "," "i" int ] ;
victory  = "majority:" int "," int ;
tiles    = entry { "," entry } ;
entry    = ( int | int "-" int ) letters ;
letters  = [ "L" | "S" ] [ "E" | "F" | "O" | "T" | "C" ] [ "R" | "Y" | "G" | "B" | "N" ] [ "p" | "w" | "k" ] ;
```

Letters may appear in any order; each class at most once. Serialisation writes them in the order
terrain, cover, owner, unit.

## Fields

| Key | Range | Default | Meaning |
|---|---|---|---|
| `n` | 2–12 | required | hex-sphere frequency; the sphere has `10n² + 2` tiles |
| `seed` | 0–4294967295 | 1 | world seed; `0` generates nothing (all sea) and the map comes entirely from `tiles` |
| `players` | 2–4 | 2 | factions in play, taken in turn order from Red |
| `human` | letters | `R` | factions controlled by people; all others in play are AI |
| `ai` | `F:h0-100,i0-3` | h50, i1 | hostility and intelligence of one AI faction; repeatable; unlisted AI factions use defaults |
| `land` | 0–100 | 42 | land share in percent for generation (prototype `LAND_FRACTION`) |
| `forest` | 0–100 | 10 | forest growth chance in percent per forest tile per turn (R-FOR-03) |
| `victory` | `majority:<percent>,<turns>` | 60,3 | majority rule parameters (R-VIC-02) |
| `tiles` | entries | none | diffs applied in order after generation |
| `name` | text | none | label shown in menus |

## Tile entries

| Letter | Meaning |
|---|---|
| `L`, `S` | terrain land / sea; `S` also clears cover, owner and unit |
| `E`, `F`, `O`, `T`, `C` | cover empty, field, forest, town, capital |
| `R`, `Y`, `G`, `B`, `N` | owner Red, Yellow, Green, Blue, none |
| `p`, `w`, `k` | unit pawn, warrior, knight, owned by the tile's owner (ignored on unowned tiles) |

Tile ids are hex-sphere indices for the given `n`; `fixtures/hexsphere/n*.json` pins them, so a
level shared today loads the same map in every future version. Ranges `a-b` are inclusive.

## Errors

Unknown keys, duplicate keys (except `ai`), out-of-range values, malformed entries, ids beyond
`10n² + 2`, factions not in play, a faction both human and AI, and a resulting world that fails
`WorldSnapshot::validate` (for example a capital without an owner) are all errors that name the
offending field or tile. Nothing is silently ignored.

## Editor (M4)

The MVP editor is a CLI in `xtask` (or a `cl-level` binary) that prints the string for a seed with
optional edits and validates pasted strings; the in-game editor and level sharing come later.
