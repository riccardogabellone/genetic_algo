<img src="assets/logo/logo.svg" alt="" width="96" align="right">

# genetic_algo

An agent-based evolution sandbox in Rust. Individuals roam a grid freely, eat, fight, reproduce and die, and heritable traits such as smartness evolve by natural selection. Optional rules add costs, vision, evolving speed, fights and elected leaders, and every rule can be tuned live in a graphical UI.

# Rules
Every numeric value is in `src/config.rs`. Defaults are in parentheses.

- each individual has a position (x, y) on the grid; the map has hard edges
- each individual has a speed, the radius r (1): the 8 * r cells of the square ring at distance r are its possible destinations
- each individual has a heritable "smartness" in [0, 100]: the chance, each turn, of deciding instead of acting at random
  - below the MIN threshold (1) it always acts at random
- each tick, every individual alive at the start of the tick acts exactly once, in random order; newborns act from the next tick

## Actions, by priority
1. **Reproduction** (highest priority, for everyone regardless of smartness): needs a partner of the opposite sex within distance r, both eligible, and a free adjacent cell for the newborn
   - eligible = "age" >= maturity (5), cooldown over (3 ticks), "health" > reproduction cost (10) + turn cost (1)
   - succeeds with probability equal to the mean of the parents' "reproduction" traits
2. **When it decides** (smartness):
   - eats FOOD if hungry, that is "health" < threshold (70) (high priority)
   - otherwise moves to a random free cell; if there is none, it stays put
   - never steps on DEATH
3. **When it acts at random**: picks any of its possible cells and takes the effect
   - free -> moves there
   - FOOD -> eats, even when full (it can die of overeating)
   - DEATH -> loses "health" (15) and stays where it is; DEATH is not consumed
   - WALL or another individual -> stays put

## Life and death
- "health": each tick it drops by 1 without food and rises by 20 with food; at 0 or at MAX (100) the individual dies
- reproduction creates no energy: each parent gives up 10 "health" and the newborn starts with 20
- "age" rises by 1 each tick; each individual dies on reaching its own "lifespan", random in [50, 100]
- FOOD regrows each tick (cells / 32), up to a maximum of cells / 4; WALL and DEATH are permanent

## Inheritance
- the child's "smartness" and "reproduction" are drawn between the parents' values (triangular distribution)
- with probability 0.5 each trait mutates by a symmetric step (smartness: up to ±8), clamped to its scale
- "generation" = the parents' highest generation + 1

## Optional rules (off by default)
- **Smartness cost** (`--smartness-cost`): extra "health" lost each turn, proportional to smartness; paid even on turns spent eating
- **Vision** (`--vision`): a hungry individual that decides walks toward the nearest visible FOOD; the radius is proportional to smartness
- **Heritable speed** (`--max-speed`): the child inherits speed from one parent, and it can mutate by ±1
- **Speed cost** (`--speed-cost`): extra "health" per point of speed above 1, on turns that change cell

## Fighting (off by default)
- **Over food** (`--contest-fights`): a hungry individual about to eat can strike an adjacent rival that is also hungry and can reach the same FOOD; then it eats
- **For no reason** (`--random-fights`): a chance each turn to attack an individual within reach, highest at smartness 0 and zero at smartness 100
- the victim loses "health" (`--fight-damage`, 10); if it reaches 0 it dies at the start of its own turn, without acting
- the attacker loses current smartness (`--fight-smartness-loss`, 5), down to 0
- **the loss is not inherited**: each individual has an innate, heritable smartness and a current one, which drives behaviour and which fights lower

## Leaders (off by default, `--leaders`)
- each individual has a heritable **policy**: aggression (multiplies the chance of fighting) and appetite (the "health" threshold below which it seeks food)
- every `--election-interval` ticks (30), or at once if the leader dies, adults vote among 5 random candidates, plus the incumbent if eligible to run again
- nobody votes for themselves; each voter picks the policy closest to its own, adjusted by its memories
- **memory** (`--memory`, 5): at the end of a term each voter remembers whether its "health" went up or down and whether its vote went to the winner. How many terms it remembers is proportional to its current smartness
  - a good term favours policies similar to the winner's; a bad one disfavours them
  - a voter who backed someone else: a good term changes its mind, a bad one proves it right
- **judgement** (`--judge-terms`): how voters judge a finished term
  - `violence` (default): the whole population's fight rate during the term, compared with the typical rate. Everyone reaches the same verdict
  - `personal`: the change in the voter's own "health". This depends almost only on luck with food, so memory learns nothing
- **term limit** (`--max-terms`, 2): nobody can be elected more than twice in their life
- **influence** (`--leader-influence`, 0.5): each turn a follower acts on the leader's policy instead of its own with probability 0.5
- memories and terms are not inherited

## Evolved brains (off by default, `--brains`)
- each individual inherits a tiny neural network: 9 inputs, 4 hidden units, 44 weights. Its weights are a genome, not trained: founders' are random, children take each weight from one parent at random, and each may mutate (`--brain-mutation-rate`, 0.05; `--brain-mutation-step`, 8 steps of 1/16)
- when an individual thinks (see smartness), its brain chooses instead of the hand-written rules: it scores staying put and every reachable cell, and the best score wins
- for each option the brain sees what the cell holds (empty, food, wall, hazard, individual, or staying), its own hunger, and how much food and how many hazards surround that cell
- the chosen cell then has its usual effect: food is eaten, a hazard hurts, a wall stops it. An individual is attacked only when unprovoked fights are allowed
- mating still comes first, as for everyone
- **control** (`--random-brains`): every child gets a fresh random brain, so nothing can accumulate across generations

## Lifetime learning (off by default, `--learning`, needs `--brains`)
- each brain also learns from experience, by value learning: the score of an option learns to predict what doing it causes. A meal is worth its food gain, a hazard its damage, an ordinary move nothing
- every turn the individual acts on a cell teaches it, whether its brain chose the cell or it acted at random, so random turns serve as exploration
- each lesson moves the weights by `--learning-rate` (0.2) times the prediction error, by gradient descent through the network
- learned weights are `f32` and start from the inherited genome. They die with the individual: children inherit only the genome

## End of the run
The run stops at the first of these conditions:
- extinction
- mean smartness >= TARGET (80), with at least INITIAL_POPULATION / 4 individuals alive (TARGET < 100 because mutation keeps everyone from staying at the maximum)
- the tick limit (10000)

## Rendering
- each tick, the position of every individual and every element on the map is drawn
- dead individuals are not drawn, and are removed from the data structures
- colours and symbols by sex (M/F), brighter when smarter
- colours and symbols per element: FOOD `*`, WALL `#`, DEATH `X`
- output to the console (ASCII) and to a file (GIF), plus a final chart (PNG) of smartness and population

# Install
The graphical app has installers for Linux (`.deb`, `.AppImage`), Windows (`-setup.exe`, `.msi`, portable `.zip`) and macOS (`.dmg`, Apple silicon) on the [releases page](https://github.com/riccardogabellone/genetic_algo/releases). They are not code-signed yet, so macOS and Windows warn on first launch; [packaging/README.md](packaging/README.md) explains how to get past the warning, and how the packages are built.

# Usage
```sh
cargo run --release                                    # live map, then results.png
cargo run --release -- --seed 42 --delay-ms 0          # replay a run, as fast as possible
cargo run --release -- --headless --gif run.gif        # no live map; record a GIF
cargo run --release -- --help                          # all options
```
Every run prints its seed. The same seed and build replay the same run.

Graphical UI, with live rule sliders and charts:
```sh
sudo apt install libegl1 libegl-mesa0                  # once, on WSL/Linux
cargo run --release -p genetic_algo_ui
```

The optional rules can slow evolution past the default tick limit, so raise it:
```sh
cargo run --release -- --smartness-cost 2 --vision 5 --ticks 20000
cargo run --release -- --smartness-cost 2 --vision 5 --max-speed 4 --speed-cost 1
```

Maps of 16x16 and larger are the supported range. Smaller maps cannot sustain a population reliably: 12x12 goes extinct in about 1 run in 6, and 8x8 almost always does.

# Architecture
| Module | Responsibility |
|---|---|
| `config` | Every rule value, validated once at start-up |
| `world` | The grid: the single source of truth for positions |
| `individual` | Traits, life-cycle rules, inheritance |
| `behavior` | Chooses an action; reads the world, changes nothing |
| `simulation` | Runs ticks, applies actions, keeps the history |
| `stats` | Per-tick measurements |
| `leadership` | Policies, elections and memory of past terms |
| `brain` | Evolved neural networks and their inheritance |
| `sampling` | Triangular distribution |
| `render` | Terminal, chart and GIF front ends, sharing one `palette` |

The core does not depend on `render`, so a front end only needs `Simulation::step()`, `world()` and `history()`.

The `ui` crate is the egui front end, kept in its own crate so the core and terminal builds do not pull in egui. `controller` holds all its run logic and draws nothing; `app`, `map_view` and `charts` only draw and report input.

The app's identity lives in `ui/src/identity.rs`: its name, the ID `io.github.riccardogabellone.GeneticAlgo` and the window icon. The same ID names the Linux desktop entry in `packaging/linux/`, so the desktop matches the window to its launcher and icon. The logo is generated, not drawn: `assets/logo/` turns the source image into SVG masters, and from them the icons in `assets/icons/` (PNG sizes, Windows `.ico`, macOS `.icns`). See `assets/logo/AGENTS.md`.

# Rule experiments
Means over 8 runs of 60,000 ticks on 32x32, started from low smartness (1-5) and from high smartness (85-95).

| Rules | From low | From high | Mean speed | Population |
|---|---|---|---|---|
| Cost 2 | 91.5 | 92.2 | 1 | 229 |
| Cost 4 | 57.5 | 69.6 (not yet converged) | 1 | 170-191 |
| Cost 8 | 4.2 | all extinct | 1 | 418 |
| Vision 5, cost 2 | 91.9 | 92.3 | 1 | 229 |
| Vision 5, cost 4 | 90.7 | 91.5 | 1 | 141 |
| Vision 5, cost 8 | 4.2 | 91.2 | 1 | 418 / 79 |
| All: cost 2, vision 5, speed cost 1 | 95.3 | 95.5 | 3.1 | 139 |
| All: cost 4, vision 5, speed cost 2 | 95.8 | 95.8 | 2.7 | 85 |

Extinct runs within 20,000 ticks, out of 60: the all-rules sets lose 0 to 1 on 32x32 but 18 to 45 on 16x16. `cargo run --release --example rule_experiments -- 8 60000` reruns the from-low rule sets it includes, each with a control; the high starts and the extinction counts were one-off experiments.

Fighting, over 24 runs of 20,000 ticks on 32x32. Founders are nearly random and so reckless: extinctions happen around tick 250, and a smart start avoids them.

| Fights over food / for no reason / damage | Start | Extinct | Current | Innate | Population |
|---|---|---|---|---|---|
| 0.5 / 0.02 / 10 | Normal | 0 | 91.7 | 95.1 | 531 |
| 0 / 0.05 / 10 | Normal | 0 | 94.8 | 94.9 | 633 |
| 0 / 0.1 / 10 | Normal | 21 | 94.9 | 95.2 | 624 |
| 0 / 0.05 / 20 | Normal | 15 | 94.9 | 95.0 | 626 |
| 0 / 0.05 / 20 | Smart | 0 | 94.8 | 94.9 | 625 |
| 0 / 0.5 / 20 | Smart | 0 | 94.5 | 96.3 | 477 |
| 1 / 0 / 20 | Smart | 0 | 88.3 | 95.2 | 345 |

Smartness and population are means over the surviving runs.

Leaders, over 48 runs of 20,000 ticks on 32x32. Fights are over food 0.5, for no reason 0.02. Means are over the surviving runs, and "Extinct" is out of 48.

| Rules | Mean aggression | Fights per individual per tick | Population | Extinct |
|---|---|---|---|---|
| No fights, policies but no influence | 1.01 | 0 | 644 | 1 |
| Fights, policies but no influence | 0.56 | 0.0145 | 579 | 0 |
| Fights, influence 0.25 | 0.67 | 0.0173 | 567 | 2 |
| Fights, influence 0.5 | 0.79 | 0.0208 | 554 | 2 |
| Fights, influence 1 | 1.02 | 0.0268 | 533 | 2 |
| Fights, no policies or leaders | 1.00 | 0.0268 | 531 | 0 |

Fights select against aggression, but acting on a leader's policy hides an individual's own from selection, so leaders keep a population from evolving peace. Memory only matters when voters judge terms by the population's fight rate and live through several elections. Over 48 runs at influence 1, with the term limit lifted so short terms do not run out of candidates, the leaders' mean aggression after tick 5,000 was:

| Term length | Memories per adult | Fights overall, memory 0 | Fights overall, memory 20 | Own health, memory 20 |
|---|---|---|---|---|
| 30 ticks (default) | 0.5 | 0.957 | 0.997 | 1.020 |
| 5 ticks | 2.6 | 0.997 | 0.962 | 1.060 |
| 2 ticks | 6.4 | 1.037 | 0.919 | 1.018 |

The standard error of each mean is about 0.02. A voter's own health over a term is mostly luck with food, so judging by it never helps. A leader's aggression strongly drives the population's fight rate (correlation about 0.9), so judging by that works: memory tilts 75% of the votes it changes toward the more peaceful candidate. But with 30-tick terms an adult lives through so few elections that it has little to remember. At 2-tick terms, memory elects calmer leaders, with about 13% fewer fights.

Evolved brains, over 24 runs of 20,000 ticks on 32x32. Means of the last 2,000 ticks; no run went extinct.

| Decisions by | Smartness at tick 1,000 | Final smartness | Population | Hazard deaths | Overfed deaths |
|---|---|---|---|---|---|
| Hand-written rules | 23.1 | 94.9 | 645 | 0.8% | 0.0% |
| Evolved brains | 12.8 | 94.8 | 633 | 1.0% | 0.4% |
| Random brains, not inherited | 8.6 | 28.8 | 489 | 11.4% | 3.3% |

Starting from random weights, evolved brains reach the level of the hand-written rules, only more slowly. Without inheritance, thinking with a random brain is harmful, so selection keeps smartness low. What the brains learned, as the share preferring the first option:

| Situation | Founders | Evolved, tick 20,000 | Not inherited |
|---|---|---|---|
| Food over an empty cell when hungry | 49% | 100% | 58% |
| An empty cell over a hazard | 52% | 100% | 50% |
| Moving over staying put | 48% | 98% | 48% |
| Food over an empty cell when full | 52% | 98% | 56% |

Evolved brains became greedy, eating even when full, which explains their few overfed deaths.

Lifetime learning, over 24 runs of 20,000 ticks on 32x32. Each value has a standard error of 1 or less; no run went extinct.

| Brains | Smartness at tick 3,000 | Final smartness | Population |
|---|---|---|---|
| Evolved, no learning | 39.8 | 94.8 | 633 |
| Evolved, learning at 0.2 | 38.9 | 94.8 | 633 |
| Evolved, learning at 0.5 | 29.9 | 94.8 | 634 |
| Random each generation, no learning | 13.7 | 28.8 | 489 |
| Random each generation, learning at 0.2 | 18.5 | 49.4 | 439 |

Learning makes thinking pay even without inherited brains, but it adds nothing to brains that already evolve, and a high rate slows early evolution by overwriting good inherited weights. Within a lifetime, learners compared with their own genome shift toward food (about 20 percentage points by age 80 at rate 0.2) and away from hazards (about 6 points).

A first version learned by REINFORCE, a policy-gradient method, from the whole change in health. It learned food weakly and drifted toward hazards: it only adjusts options relative to each other, food is rewarded often and hazards are rarely tried, and the turn's fixed costs carried no information. Value learning fixed both.

# Testing
```sh
cargo test --workspace
cargo run --release --example selection_vs_control     # evolution vs a control run
```
Smartness rising is not enough to prove evolution: it starts near 0 and is clamped there, so mutation alone pushes it up. Only the gap against the control, where smartness has no effect on behaviour, is evidence of natural selection.

# Contributing
Work on a branch and open a pull request against `main`. AI coding agents should follow [AGENTS.md](AGENTS.md).

# License
[MIT](LICENSE)
