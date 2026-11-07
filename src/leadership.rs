//! Elected leaders. Everyone has a heritable policy; adults periodically vote for the
//! candidate they prefer, and followers sometimes adopt the winner's policy until the next
//! election. Smarter voters remember how past elections turned out for them.

use rand::{RngExt, seq::IndexedRandom};

use crate::{
    config::{Config, MAX_SMARTNESS},
    individual::{Individual, inherit, mutate},
    world::World,
};

pub const MAX_AGGRESSION: f32 = 2.0;
/// A health change of this share of the scale counts as a term going fully well or badly.
const FULL_OUTCOME_SWING: f32 = 0.25;

/// How an individual would like everyone to behave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Policy {
    /// Multiplies the chance of fights. Neutral is 1, within `0..=MAX_AGGRESSION`.
    pub aggression: f32,
    /// The share of maximum health below which food is wanted, within `0..=1`. Eating
    /// too soon risks overeating; eating too late risks starving.
    pub appetite: f32,
}

impl Policy {
    /// The behaviour when leaders are off.
    pub fn neutral(config: &Config) -> Self {
        Self {
            aggression: 1.0,
            appetite: config.hungry_below_health as f32 / config.max_health as f32,
        }
    }

    /// How different two policies are, from 0 (equal) to 1 (opposite corners).
    pub fn distance(self, other: Self) -> f32 {
        let aggression = (self.aggression - other.aggression).abs() / MAX_AGGRESSION;
        let appetite = (self.appetite - other.appetite).abs();
        f32::midpoint(aggression, appetite)
    }

    fn similarity(self, other: Self) -> f32 {
        1.0 - self.distance(other)
    }

    fn stance(self, config: &Config) -> Stance {
        Stance {
            hungry_below: (self.appetite * config.max_health as f32).round() as u32,
            aggression: self.aggression,
        }
    }
}

/// What a turn actually acts on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stance {
    pub hungry_below: u32,
    pub aggression: f32,
}

impl Stance {
    /// Exactly the configured rules, with no rounding through a policy.
    pub fn neutral(config: &Config) -> Self {
        Self {
            hungry_below: config.hungry_below_health,
            aggression: 1.0,
        }
    }

    /// This turn's stance: the leader's policy when followed, otherwise the individual's own.
    pub fn for_turn(
        individual: &Individual,
        leader_policy: Option<Policy>,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Self {
        if !config.leaders_enabled {
            return Self::neutral(config);
        }
        let followed =
            leader_policy.filter(|_| rng.random_bool(f64::from(config.leader_influence)));
        followed.unwrap_or(individual.civics.policy).stance(config)
    }

    /// The individual's own stance, for others judging whether it wants food.
    pub fn own(individual: &Individual, config: &Config) -> Self {
        if config.leaders_enabled {
            individual.civics.policy.stance(config)
        } else {
            Self::neutral(config)
        }
    }
}

/// A vote cast, waiting to be judged when the term it started is over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vote {
    pub chosen_id: u64,
    pub chosen: Policy,
    pub health_when_cast: u32,
}

/// How one past election turned out for this voter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermMemory {
    pub chosen: Policy,
    pub winner: Policy,
    /// Whether the voter's choice was the one elected.
    pub agreed: bool,
    /// From -1 (the term went badly for this voter) to 1 (it went well).
    pub outcome: f32,
}

/// The part of political life that only exists for those who have voted or stood.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CivicRecord {
    pub terms_served: u32,
    pub pending_vote: Option<Vote>,
    /// Oldest first, at most `Config::max_memory` long.
    pub memories: Vec<TermMemory>,
}

/// An individual's political life. The record is allocated on first use, because
/// `Individual` must stay small: see the size test in `individual.rs`.
#[derive(Debug, Clone, PartialEq)]
pub struct Civics {
    pub policy: Policy,
    record: Option<Box<CivicRecord>>,
}

impl Civics {
    fn with_policy(policy: Policy) -> Self {
        Self {
            policy,
            record: None,
        }
    }

    pub fn terms_served(&self) -> u32 {
        self.record.as_ref().map_or(0, |record| record.terms_served)
    }

    pub fn pending_vote(&self) -> Option<&Vote> {
        self.record
            .as_ref()
            .and_then(|record| record.pending_vote.as_ref())
    }

    pub fn memories(&self) -> &[TermMemory] {
        self.record.as_ref().map_or(&[], |record| &record.memories)
    }

    pub fn record_mut(&mut self) -> &mut CivicRecord {
        self.record.get_or_insert_default()
    }

    /// A neutral citizen, for scripted scenarios and when leaders are off.
    pub fn neutral(config: &Config) -> Self {
        Self::with_policy(Policy::neutral(config))
    }

    /// Founders draw a policy only when leaders are on, so runs without leaders use no
    /// extra random numbers and replay exactly as they did before this rule existed.
    pub fn founder(config: &Config, rng: &mut impl RngExt) -> Self {
        if !config.leaders_enabled {
            return Self::neutral(config);
        }
        Self::with_policy(Policy {
            aggression: config.founder_aggression.sample(rng),
            appetite: config.founder_appetite.sample(rng),
        })
    }

    /// Children inherit a policy between their parents' and may mutate it. Memories and
    /// terms are personal, and are not inherited.
    pub fn inherited(
        parent_a: &Individual,
        parent_b: &Individual,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Self {
        let (a, b) = (parent_a.civics.policy, parent_b.civics.policy);
        if !config.leaders_enabled {
            return Self::with_policy(a);
        }
        let step = config.policy_mutation_step;
        let rate = config.mutation_rate;
        Self::with_policy(Policy {
            aggression: mutate(
                inherit(a.aggression, b.aggression, rng),
                step * MAX_AGGRESSION,
                MAX_AGGRESSION,
                rate,
                rng,
            ),
            appetite: mutate(inherit(a.appetite, b.appetite, rng), step, 1.0, rate, rng),
        })
    }
}

/// How many past elections this individual can recall. Smarter means more; fights lower it.
pub fn recall_depth(smartness: f32, config: &Config) -> usize {
    (smartness / MAX_SMARTNESS * config.max_memory as f32).round() as usize
}

/// How much `voter` likes a candidate's policy. Closeness to the voter's own policy is the
/// base. Each recalled election then counts for or against a similar policy: a good term
/// favours its winner's policy and a bad one counts against it. A voter who chose someone
/// else also judges that choice, which a good term discredits and a bad one vindicates.
pub fn preference(voter: &Individual, candidate: Policy, config: &Config) -> f32 {
    let own = voter.civics.policy;
    let recalled = voter
        .civics
        .memories()
        .iter()
        .rev()
        .take(recall_depth(voter.smartness, config));
    let mut score = -own.distance(candidate);
    for memory in recalled {
        score += memory.outcome * candidate.similarity(memory.winner);
        if !memory.agreed {
            score -= memory.outcome * candidate.similarity(memory.chosen);
        }
    }
    score
}

fn can_vote(individual: &Individual, config: &Config) -> bool {
    individual.age >= config.maturity_age && individual.health > 0
}

fn can_stand(individual: &Individual, config: &Config) -> bool {
    can_vote(individual, config) && individual.civics.terms_served() < config.max_terms
}

/// Who leads now, and how the last term is remembered.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Leadership {
    leader: Option<u64>,
    /// The last winner, kept after death so its term can still be judged.
    last_winner: Option<(u64, Policy)>,
    term_started: u32,
}

impl Leadership {
    pub fn leader(&self) -> Option<u64> {
        self.leader
    }

    pub fn term_started(&self) -> u32 {
        self.term_started
    }

    /// The current leader's policy.
    pub fn policy(&self) -> Option<Policy> {
        self.leader.and(self.last_winner.map(|(_, policy)| policy))
    }

    /// Due when the term is up, or at once when there is no leader.
    pub fn election_due(&self, tick: u32, config: &Config) -> bool {
        config.leaders_enabled
            && (self.leader.is_none()
                || tick.saturating_sub(self.term_started) >= config.election_interval)
    }

    pub fn record_death(&mut self, id: u64) {
        if self.leader == Some(id) {
            self.leader = None;
        }
    }

    /// Runs an election and returns whether someone won. Nothing changes if there is no
    /// candidate, or if nobody else is there to vote for them.
    pub fn hold_election(
        &mut self,
        world: &mut World,
        tick: u32,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> bool {
        let voters = world
            .individuals()
            .filter(|(_, individual)| can_vote(individual, config))
            .count();
        let ballot = self.draw_ballot(world, config, rng);
        if ballot.is_empty() || voters < 2 {
            return false;
        }

        let mut votes = vec![0_u32; ballot.len()];
        let closing = self.last_winner;
        for (_, voter) in world.individuals_mut() {
            if !can_vote(voter, config) {
                continue;
            }
            remember_term(voter, closing, config);
            if let Some(choice) = choose(voter, &ballot, config) {
                let (id, policy) = ballot[choice];
                votes[choice] += 1;
                let health_when_cast = voter.health;
                voter.civics.record_mut().pending_vote = Some(Vote {
                    chosen_id: id,
                    chosen: policy,
                    health_when_cast,
                });
            }
        }

        // The ballot is in random order, so taking the first of equals is fair.
        let mut winner = 0;
        for (index, count) in votes.iter().enumerate() {
            if *count > votes[winner] {
                winner = index;
            }
        }
        let (winner_id, winner_policy) = ballot[winner];
        for (_, individual) in world.individuals_mut() {
            if individual.id == winner_id {
                individual.civics.record_mut().terms_served += 1;
            }
        }
        self.leader = Some(winner_id);
        self.last_winner = Some((winner_id, winner_policy));
        self.term_started = tick;
        true
    }

    /// A random few who may stand, plus the incumbent if it still may.
    fn draw_ballot(
        &self,
        world: &World,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Vec<(u64, Policy)> {
        let eligible: Vec<(u64, Policy)> = world
            .individuals()
            .filter(|(_, individual)| can_stand(individual, config))
            .map(|(_, individual)| (individual.id, individual.civics.policy))
            .collect();
        let mut ballot: Vec<(u64, Policy)> =
            eligible.sample(rng, config.ballot_size).copied().collect();
        if let Some(incumbent) = self.leader
            && !ballot.iter().any(|(id, _)| *id == incumbent)
            && let Some(entry) = eligible.iter().find(|(id, _)| *id == incumbent)
        {
            ballot.push(*entry);
        }
        ballot
    }
}

/// The voter's favourite on the ballot. Nobody votes for themselves.
fn choose(voter: &Individual, ballot: &[(u64, Policy)], config: &Config) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, (id, policy)) in ballot.iter().enumerate() {
        if *id == voter.id {
            continue;
        }
        let score = preference(voter, *policy, config);
        if best.is_none_or(|(_, top)| score > top) {
            best = Some((index, score));
        }
    }
    best.map(|(index, _)| index)
}

/// Turns the voter's last vote into a memory, judged by how its health changed since.
fn remember_term(voter: &mut Individual, closing: Option<(u64, Policy)>, config: &Config) {
    let health = voter.health as f32;
    let record = voter.civics.record_mut();
    let (Some(vote), Some((winner_id, winner))) = (record.pending_vote.take(), closing) else {
        return;
    };
    let change = health - vote.health_when_cast as f32;
    let full_swing = config.max_health as f32 * FULL_OUTCOME_SWING;
    record.memories.push(TermMemory {
        chosen: vote.chosen,
        winner,
        agreed: vote.chosen_id == winner_id,
        outcome: (change / full_swing).clamp(-1.0, 1.0),
    });
    let excess = record.memories.len().saturating_sub(config.max_memory);
    record.memories.drain(..excess);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::individual::Gender;
    use rand::{SeedableRng, rngs::StdRng};

    fn config() -> Config {
        Config {
            leaders_enabled: true,
            ..Config::default()
        }
    }

    fn voter(smartness: f32, policy: Policy) -> Individual {
        let mut rng = StdRng::seed_from_u64(1);
        let mut individual = Individual::founder(0, Gender::Male, &config(), &mut rng);
        individual.smartness = smartness;
        individual.civics.policy = policy;
        individual
    }

    const DOVE: Policy = Policy {
        aggression: 0.0,
        appetite: 0.5,
    };
    const HAWK: Policy = Policy {
        aggression: 2.0,
        appetite: 0.5,
    };

    fn memory(winner: Policy, chosen: Policy, agreed: bool, outcome: f32) -> TermMemory {
        TermMemory {
            chosen,
            winner,
            agreed,
            outcome,
        }
    }

    #[test]
    fn distance_runs_from_zero_to_one() {
        let far = Policy {
            aggression: 2.0,
            appetite: 1.0,
        };
        let near = Policy {
            aggression: 0.0,
            appetite: 0.0,
        };
        assert_eq!(near.distance(near), 0.0);
        assert_eq!(near.distance(far), 1.0);
        assert_eq!(far.distance(near), 1.0);
    }

    #[test]
    fn neutral_stance_matches_the_rules_exactly() {
        let config = Config::default();
        assert_eq!(
            Stance::neutral(&config).hungry_below,
            config.hungry_below_health
        );
        assert_eq!(Policy::neutral(&config).stance(&config).hungry_below, 70);
    }

    #[test]
    fn voters_prefer_the_closest_policy_without_memories() {
        let config = config();
        let dove = voter(100.0, DOVE);
        assert!(preference(&dove, DOVE, &config) > preference(&dove, HAWK, &config));
    }

    #[test]
    fn recall_depth_grows_with_smartness() {
        let config = config();
        assert_eq!(recall_depth(0.0, &config), 0);
        assert_eq!(recall_depth(100.0, &config), config.max_memory);
        assert!(recall_depth(50.0, &config) < config.max_memory);
    }

    /// A dove who elected a dove, and suffered for it, turns against doves.
    #[test]
    fn a_bad_term_counts_against_its_winner() {
        let config = config();
        let mut dove = voter(100.0, DOVE);
        for _ in 0..2 {
            dove.civics
                .record_mut()
                .memories
                .push(memory(DOVE, DOVE, true, -1.0));
        }
        assert!(preference(&dove, HAWK, &config) > preference(&dove, DOVE, &config));
    }

    /// A dove who voted against a hawk that then did well for it warms to hawks.
    #[test]
    fn a_good_term_under_the_other_side_changes_a_voters_mind() {
        let config = config();
        let mut dove = voter(100.0, DOVE);
        dove.civics
            .record_mut()
            .memories
            .push(memory(HAWK, DOVE, false, 1.0));
        assert!(preference(&dove, HAWK, &config) > preference(&dove, DOVE, &config));
    }

    /// A hawk who voted for a dove, and suffered under the winning hawk, feels vindicated.
    #[test]
    fn a_bad_term_vindicates_those_who_voted_against_it() {
        let config = config();
        let mut hawk = voter(100.0, HAWK);
        hawk.civics
            .record_mut()
            .memories
            .push(memory(HAWK, DOVE, false, -1.0));
        assert!(preference(&hawk, DOVE, &config) > preference(&hawk, HAWK, &config));
    }

    #[test]
    fn without_smartness_nothing_is_recalled() {
        let config = config();
        let mut forgetful = voter(0.0, DOVE);
        forgetful
            .civics
            .record_mut()
            .memories
            .push(memory(DOVE, DOVE, true, -1.0));
        assert!(preference(&forgetful, DOVE, &config) > preference(&forgetful, HAWK, &config));
    }

    #[test]
    fn only_the_most_recent_elections_are_recalled() {
        let config = Config {
            max_memory: 10,
            ..config()
        };
        // Depth 1 of 10: only the newest memory counts, which cancels nothing older.
        let mut slow = voter(10.0, DOVE);
        slow.civics
            .record_mut()
            .memories
            .push(memory(HAWK, HAWK, true, 1.0));
        slow.civics
            .record_mut()
            .memories
            .push(memory(DOVE, DOVE, true, 1.0));
        assert_eq!(recall_depth(slow.smartness, &config), 1);
        assert!(preference(&slow, DOVE, &config) > preference(&slow, HAWK, &config));
    }

    #[test]
    fn terms_are_judged_by_the_health_change_and_clamped() {
        let config = config();
        let closing = Some((7, HAWK));
        for (before, after, expected) in [
            (50, 75, 1.0),
            (50, 25, -1.0),
            (50, 60, 0.4),
            (10, 99, 1.0),
            (99, 1, -1.0),
        ] {
            let mut judge = voter(100.0, DOVE);
            judge.health = after;
            let health_when_cast = before;
            judge.civics.record_mut().pending_vote = Some(Vote {
                chosen_id: 7,
                chosen: HAWK,
                health_when_cast,
            });
            remember_term(&mut judge, closing, &config);
            let remembered = judge.civics.memories()[0];
            assert!(
                (remembered.outcome - expected).abs() < 1e-6,
                "{before}->{after}"
            );
            assert!(remembered.agreed);
            assert_eq!(judge.civics.pending_vote(), None);
        }
    }

    #[test]
    fn voting_for_someone_else_is_remembered_as_disagreement() {
        let config = config();
        let mut judge = voter(100.0, DOVE);
        let health_when_cast = judge.health;
        judge.civics.record_mut().pending_vote = Some(Vote {
            chosen_id: 1,
            chosen: DOVE,
            health_when_cast,
        });
        remember_term(&mut judge, Some((2, HAWK)), &config);
        assert!(!judge.civics.memories()[0].agreed);
    }

    #[test]
    fn memories_are_capped_and_keep_the_newest() {
        let config = Config {
            max_memory: 3,
            ..config()
        };
        let mut judge = voter(100.0, DOVE);
        for tenth in 0..8 {
            judge.health = 50 + tenth;
            let health_when_cast = 50;
            judge.civics.record_mut().pending_vote = Some(Vote {
                chosen_id: 1,
                chosen: DOVE,
                health_when_cast,
            });
            remember_term(&mut judge, Some((1, DOVE)), &config);
        }
        assert_eq!(judge.civics.memories().len(), 3);
        assert!(judge.civics.memories()[2].outcome > judge.civics.memories()[0].outcome);
    }

    #[test]
    fn zero_memory_keeps_nothing() {
        let config = Config {
            max_memory: 0,
            ..config()
        };
        let mut judge = voter(100.0, DOVE);
        let health_when_cast = 50;
        judge.civics.record_mut().pending_vote = Some(Vote {
            chosen_id: 1,
            chosen: DOVE,
            health_when_cast,
        });
        remember_term(&mut judge, Some((1, DOVE)), &config);
        assert!(judge.civics.memories().is_empty());
    }

    #[test]
    fn nobody_votes_for_themselves() {
        let config = config();
        let me = voter(100.0, DOVE);
        assert_eq!(choose(&me, &[(me.id, DOVE)], &config), None);
        assert_eq!(choose(&me, &[(me.id, DOVE), (99, HAWK)], &config), Some(1));
    }

    #[test]
    fn inherited_policies_stay_in_range_and_start_a_clean_record() {
        let config = Config {
            mutation_rate: 1.0,
            policy_mutation_step: 1.0,
            ..config()
        };
        let mut rng = StdRng::seed_from_u64(3);
        let mut parent = voter(100.0, HAWK);
        parent.civics.record_mut().terms_served = 2;
        parent
            .civics
            .record_mut()
            .memories
            .push(memory(DOVE, DOVE, true, 1.0));
        for _ in 0..500 {
            let child = Civics::inherited(&parent, &parent, &config, &mut rng);
            assert!((0.0..=MAX_AGGRESSION).contains(&child.policy.aggression));
            assert!((0.0..=1.0).contains(&child.policy.appetite));
            assert_eq!(child.terms_served(), 0);
            assert!(child.memories().is_empty());
            assert_eq!(child.pending_vote(), None);
        }
    }

    #[test]
    fn without_leaders_policies_are_copied_without_touching_the_random_stream() {
        let config = Config::default();
        let parent = voter(100.0, HAWK);
        let mut rng = StdRng::seed_from_u64(4);
        let mut untouched = StdRng::seed_from_u64(4);
        let child = Civics::inherited(&parent, &parent, &config, &mut rng);
        assert_eq!(child.policy, HAWK);
        let _ = Civics::founder(&config, &mut rng);
        assert_eq!(rng.random::<u64>(), untouched.random::<u64>());
    }
}
