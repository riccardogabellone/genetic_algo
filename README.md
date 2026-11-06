---
applyTo: '**'
---
Coding standards, domain knowledge, and preferences that AI should follow.

# Coding Standards and Preferences
## General Guidelines
- Use clear and descriptive variable names.
- Write comments to explain complex logic.
- Keep functions small and focused on a single task.

# Domain Knowledge
We have to see little individual agents evolving in a genetic algo like environment, with the ability to do some actions to step forward the generations.

## Program features
Tutti i valori numerici sono in `src/config.rs`. Tra parentesi i default.

- ogni Individuo ha una posizione (x, y) nello spazio; la mappa ha bordi rigidi
- ogni Individuo ha una velocità -> radius (r) (1) -> le 8 * r caselle del quadrato a distanza r sono le possibili destinazioni
- ogni Individuo ha una "smartness" ereditabile in [0, 100]: è la probabilità, a ogni turno, di decidere invece di agire a caso
  - sotto la soglia MIN (1) agisce sempre a caso
- ogni tick, tutti gli Individui vivi all'inizio del tick agiscono una sola volta, in ordine casuale; i neonati agiscono dal tick successivo

### Azioni, in ordine di priorità
1. **Riproduzione** (PRIORITY.MAX, vale per tutti, a prescindere dalla smartness): serve un partner del sesso opposto a distanza r, entrambi idonei, e una casella libera adiacente per il neonato
   - idoneo = "age" >= maturità (5), cooldown finito (3 tick), "health" > costo riproduzione (10) + costo del turno (1)
   - riesce con probabilità pari alla media dei tratti "reproduction" dei genitori
2. **Se decide** (smartness):
   - mangia FOOD se ha fame, cioè "health" < soglia (70) (PRIORITY.HIGH)
   - altrimenti si sposta su una casella libera a caso; se non ce ne sono, resta fermo
   - non tocca mai DEATH
3. **Se agisce a caso**: sceglie una qualsiasi delle caselle possibili e ne subisce l'effetto
   - libera -> si sposta
   - FOOD -> mangia, anche se è sazio (può morire per eccesso di cibo)
   - DEATH -> perde "health" (15) e resta dove si trova; DEATH non si consuma
   - WALL o altro Individuo -> resta fermo

### Vita e morte
- "health": ogni tick diminuisce di 1 se non mangia, aumenta di 20 se mangia; a 0 o a MAX (100) muore
- la riproduzione non crea energia: ogni genitore cede 10 di "health" e il neonato parte con 20
- "age" aumenta di 1 a ogni tick; ogni Individuo muore al raggiungimento della propria "lifespan", casuale in [50, 100]
- FOOD ricresce a ogni tick (celle / 32), fino a un massimo di celle / 4; WALL e DEATH sono permanenti

### Ereditarietà
- "smartness" e "reproduction" del figlio sono estratte tra i valori dei genitori (distribuzione triangolare)
- con probabilità 0.5 ogni tratto muta di un passo simmetrico (smartness: fino a ±8), limitato alla sua scala
- "generation" = generazione più alta dei genitori + 1

### Regole opzionali (spente di default)
- **Costo della smartness** (`--smartness-cost`): "health" extra persa a ogni turno, proporzionale alla smartness; la paga anche chi mangia
- **Visione** (`--vision`): chi decide e ha fame cammina verso il FOOD visibile più vicino; il raggio è proporzionale alla smartness
- **Velocità ereditabile** (`--max-speed`): il figlio eredita la velocità da un genitore, e può mutare di ±1
- **Costo della velocità** (`--speed-cost`): "health" extra per ogni punto di velocità sopra 1, nei turni in cui cambia casella

### Fine della simulazione
La simulazione termina alla prima di queste condizioni:
- estinzione
- smartness media >= TARGET (80), con almeno POP_INIZIALE / 4 Individui vivi (TARGET < 100 perché le mutazioni impediscono a tutti di restare al massimo)
- limite di tick (10000)

## PLOT
- ogni tick, plotto la posizione di ogni Individuo e di ogni elemento sulla mappa
- non compare un Individuo se è morto (rimosso comunque dalla struttura dati)
- colori e simboli per sesso/genere (M/F), più luminosi se più "smart"
- colori e simboli per ELEMENT: FOOD `*`, WALL `#`, DEATH `X`
- PLOT su console (ASCII) e su file (GIF), più un grafico finale (PNG) di smartness e popolazione

# Usage
```sh
cargo run --release                                    # live map, then results.png
cargo run --release -- --seed 42 --delay-ms 0          # replay a run, as fast as possible
cargo run --release -- --headless --gif run.gif        # no live map; record a GIF
cargo run --release -- --help                          # all options
```
Every run prints its seed. The same seed and build replay the same run.

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
| `sampling` | Triangular distribution |
| `render` | Terminal, chart and GIF front ends, sharing one `palette` |

The core does not depend on `render`, so a graphical front end only needs `Simulation::step()`, `world()` and `history()`.

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

# Testing
```sh
cargo test
cargo run --release --example selection_vs_control     # evolution vs a control run
```
Smartness rising is not enough to prove evolution: it starts near 0 and is clamped there, so mutation alone pushes it up. Only the gap against the control, where smartness has no effect on behaviour, is evidence of natural selection.
