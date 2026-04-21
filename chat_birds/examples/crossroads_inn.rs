use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use chat_birds::*;
// ══════════════════════════════════════════════════════════════════════════════
//  THE CROSSROADS INN — example
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
enum Mood {
    Joyful,
    Neutral,
    Wary,
    Furious,
}
impl_state!(Mood);

#[derive(Clone, Debug)]
struct Health(pub f32);
#[derive(Clone, Debug)]
struct Hunger(pub f32);
#[derive(Clone, Debug)]
struct Gold(pub u32);
/// Composite alias: health < 40
#[derive(Clone, Debug)]
struct Injured;
/// Composite alias: hunger > 80
#[derive(Clone, Debug)]
struct Starving;

impl_state!(Health);
impl_state!(Hunger);
impl_state!(Gold);
impl_state!(Injured);
impl_state!(Starving);

impl std::fmt::Display for Mood {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Mood::Joyful => write!(f, "Joyful"),
            Mood::Neutral => write!(f, "Neutral"),
            Mood::Wary => write!(f, "Wary"),
            Mood::Furious => write!(f, "Furious"),
        }
    }
}

enum Personality {
    Innkeeper,
    Bard,
    Mercenary,
}

struct Npc {
    id: AgentId,
    name: String,
    personality: Personality,
    states: StateMap,
    beliefs: BeliefStore,
    trust: HashMap<AgentId, f32>,
}

impl Npc {
    fn new(id: u32, name: &str, personality: Personality) -> Self {
        let mut states = StateMap::new();
        states.insert(Health(100.0));
        states.insert(Hunger(20.0));
        states.insert(Mood::Neutral);
        states.insert(Gold(10));
        Npc {
            id: AgentId(id),
            name: name.to_string(),
            personality,
            states,
            beliefs: BeliefStore::new(),
            trust: HashMap::new(),
        }
    }

    fn set_trust(&mut self, other: AgentId, t: f32) {
        self.trust.insert(other, t.clamp(0.0, 1.0));
    }
    fn get_trust(&self, other: AgentId) -> f32 {
        self.trust.get(&other).copied().unwrap_or(0.5)
    }

    fn pack_own_states(&self) -> BeliefStore {
        let mut store = BeliefStore::new();
        let bmap = store.get_or_insert(&self.name.as_str());
        for (tid, state) in &self.states.0 {
            bmap.0.entry(*tid).or_default().push(BeliefEntry {
                state: state.clone_box(),
                certainty: 255,
                probability: Probability::Always,
                source: BeliefSource::Myself,
                temporal: Temporal::Tense(Tense::Present),
            });
        }
        store
    }

    fn generate_utterance(&self, target: &str) -> String {
        let mood = self.states.get::<Mood>().cloned().unwrap_or(Mood::Neutral);
        let hunger = self.states.get::<Hunger>().map(|h| h.0).unwrap_or(0.0);
        let health = self.states.get::<Health>().map(|h| h.0).unwrap_or(100.0);
        let gold = self.states.get::<Gold>().map(|g| g.0).unwrap_or(0);
        match self.personality {
            Personality::Innkeeper => match (&mood, hunger > 70.0, health < 40.0, gold < 5) {
                (_, true, _, _) => format!(
                    "{}: \"Excuse me, {}, but I haven't eaten since dawn.\"",
                    self.name, target
                ),
                (_, _, true, _) => format!(
                    "{}: \"Not at my best today, {}. What do you need?\"",
                    self.name, target
                ),
                (_, _, _, true) => format!(
                    "{}: \"Slow night, {}. Barely keeping the fires going.\"",
                    self.name, target
                ),
                (Mood::Joyful, ..) => format!(
                    "{}: \"Welcome, {}! First round is on me!\"",
                    self.name, target
                ),
                (Mood::Wary, ..) => format!(
                    "{}: \"I'll be watching you, {}. Don't make me regret this.\"",
                    self.name, target
                ),
                (Mood::Furious, ..) => format!(
                    "{}: \"You've got some nerve showing up here, {}.\"",
                    self.name, target
                ),
                _ => format!("{}: \"What'll it be, {}?\"", self.name, target),
            },
            Personality::Bard => match (&mood, hunger > 70.0) {
                (_, true) => format!(
                    "{}: \"Even muses must eat, {}. A ballad after supper?\"",
                    self.name, target
                ),
                (Mood::Joyful, _) => format!(
                    "{}: \"{}! You arrive like a chorus after a long verse!\"",
                    self.name, target
                ),
                (Mood::Furious, _) => format!(
                    "{}: \"Not now, {}. Someone stole my best ballad.\"",
                    self.name, target
                ),
                (Mood::Wary, _) => format!(
                    "{}: \"I've heard stories about you, {}. Interesting ones.\"",
                    self.name, target
                ),
                _ => format!(
                    "{}: \"Lovely to see you, {}. Sit, and I'll play something fitting.\"",
                    self.name, target
                ),
            },
            Personality::Mercenary => match (&mood, health < 40.0, hunger > 70.0) {
                (_, true, _) => format!(
                    "{}: \"Took hits today, {}. Still standing.\"",
                    self.name, target
                ),
                (_, _, true) => format!("{}: \"Hungry. Talk later, {}.\"", self.name, target),
                (Mood::Joyful, ..) => format!(
                    "{}: \"Good contract today. Buy you a drink, {}?\"",
                    self.name, target
                ),
                (Mood::Furious, ..) => format!(
                    "{}: *cracks knuckles* \"You want trouble, {}?\"",
                    self.name, target
                ),
                (Mood::Wary, ..) => {
                    format!("{}: \"Something's off about you, {}.\"", self.name, target)
                }
                _ => format!("{}: \"Speak, {}.\"", self.name, target),
            },
        }
    }

    fn describe_entry(e: &BeliefEntry) -> String {
        let s = &e.state;
        let cert = (u16::from(e.certainty) * 100) / 255;
        let src = match &e.source {
            BeliefSource::Myself => "self".to_string(),
            BeliefSource::Agent(id) => format!("agent:{}", id.0),
            BeliefSource::Inferred => "inferred".to_string(),
        };
        let val = if let Some(h) = s.as_any().downcast_ref::<Health>() {
            format!("health={:.0}", h.0)
        } else if let Some(h) = s.as_any().downcast_ref::<Hunger>() {
            format!("hunger={:.0}", h.0)
        } else if let Some(m) = s.as_any().downcast_ref::<Mood>() {
            format!("mood={}", m)
        } else if let Some(g) = s.as_any().downcast_ref::<Gold>() {
            format!("gold={}", g.0)
        } else if s.as_any().downcast_ref::<Injured>().is_some() {
            "injured".to_string()
        } else if s.as_any().downcast_ref::<Starving>().is_some() {
            "starving".to_string()
        } else if let Some(nb) = s.as_any().downcast_ref::<NestedBelief>() {
            let n: usize = nb
                .store
                .0
                .values()
                .map(|bm| bm.0.values().map(|v| v.len()).sum::<usize>())
                .sum();
            format!("nested_beliefs({} entries)", n)
        } else {
            "?".to_string()
        };
        format!("{} [{}% via {}]", val, cert, src)
    }

    fn print_status(&self) {
        let health = self.states.get::<Health>().map(|h| h.0).unwrap_or(0.0);
        let hunger = self.states.get::<Hunger>().map(|h| h.0).unwrap_or(0.0);
        let mood = self.states.get::<Mood>().cloned().unwrap_or(Mood::Neutral);
        let gold = self.states.get::<Gold>().map(|g| g.0).unwrap_or(0);
        println!("┌─ {} (id:{})", self.name, self.id.0);
        println!(
            "│  HP {:>5.0}  Hunger {:>5.0}  Mood {:>8}  Gold {:>4}",
            health, hunger, mood, gold
        );
        if !self.trust.is_empty() {
            let ts: Vec<String> = self
                .trust
                .iter()
                .map(|(id, t)| format!("agent{}={:.0}%", id.0, t * 100.0))
                .collect();
            println!("│  Trust  : {}", ts.join(", "));
        }
        if self.beliefs.0.is_empty() {
            println!("│  Beliefs: (none)");
        } else {
            println!("│  Beliefs:");
            for (key, bmap) in &self.beliefs.0 {
                for (_, entries) in &bmap.0 {
                    for e in entries {
                        println!("│    '{}': {}", key, Self::describe_entry(e));
                    }
                }
            }
        }
        println!("└──────────────────────────────────────────────────────────");
    }
}

impl Agent for Npc {
    fn id(&self) -> AgentId {
        self.id
    }
    fn states(&self) -> &StateMap {
        &self.states
    }
    fn states_mut(&mut self) -> &mut StateMap {
        &mut self.states
    }
    fn beliefs(&self) -> &BeliefStore {
        &self.beliefs
    }
    fn beliefs_mut(&mut self) -> &mut BeliefStore {
        &mut self.beliefs
    }

    fn on_message(&mut self, msg: Message) -> Vec<Message> {
        let from = msg.from;
        if let Some(u) = &msg.utterance {
            println!("  [{}] hears: {}", self.name, u);
        }
        self.merge_payload(from, msg.payload);
        println!("  [{}] updated beliefs from agent {}.", self.name, from.0);
        vec![]
    }

    fn decay(&mut self) {
        let mut forgotten = 0usize;
        let mut degraded = 0usize;
        for (_, bmap) in self.beliefs_mut().0.iter_mut() {
            for (_, entries) in bmap.0.iter_mut() {
                for e in entries.iter_mut() {
                    e.certainty = e.certainty.saturating_sub(38);
                    if e.certainty < 102 {
                        if let BeliefSource::Agent(_) = e.source {
                            e.source = BeliefSource::Inferred;
                            degraded += 1;
                        }
                    }
                }
                let before = entries.len();
                entries.retain(|e| e.certainty > 0);
                forgotten += before - entries.len();
            }
        }
        println!(
            "  [{}] memory fades. {} forgotten, {} sources degraded to 'inferred'.",
            self.name, forgotten, degraded
        );
    }

    fn observe_outcome(&mut self, agent: AgentId, confirmed: bool) {
        let t = self.trust.entry(agent).or_insert(0.5);
        if confirmed {
            *t = (*t + 0.05).min(1.0);
        } else {
            *t = (*t - 0.10).max(0.0);
        }
        println!(
            "  [{}] trust in agent {} → {:.0}%.",
            self.name,
            agent.0,
            *t * 100.0
        );
    }

    fn resolve_belief(
        &self,
        _key: &str,
        from: AgentId,
        existing: Vec<BeliefEntry>,
        mut incoming: BeliefEntry,
    ) -> Vec<BeliefEntry> {
        let scaled = (f32::from(incoming.certainty) * self.get_trust(from)).round();
        incoming.certainty = scaled.clamp(0.0, 255.0) as u8;
        let tid = incoming.state.as_any().type_id();
        let cert = incoming.certainty;
        let mut result: Vec<BeliefEntry> = existing
            .into_iter()
            .filter(|e| !(e.state.as_any().type_id() == tid && cert >= e.certainty))
            .collect();
        result.push(incoming);
        result
    }
}

struct Inn {
    agents: HashMap<AgentId, Npc>,
    names: HashMap<String, AgentId>,
    registry: StateRegistry,
}

impl Inn {
    fn new() -> Self {
        let mut r = StateRegistry::new();
        r.register::<Health>("health");
        r.register::<Hunger>("hunger");
        r.register::<Mood>("mood");
        r.register::<Gold>("gold");
        r.register::<Injured>("injured (alias: low health)");
        r.register::<Starving>("starving (alias: high hunger)");
        r.alias::<Injured, Health>();
        r.alias::<Starving, Hunger>();
        Inn {
            agents: HashMap::new(),
            names: HashMap::new(),
            registry: r,
        }
    }

    fn add(&mut self, npc: Npc) {
        self.names.insert(npc.name.to_lowercase(), npc.id);
        self.agents.insert(npc.id, npc);
    }

    fn resolve(&self, name: &str) -> Option<AgentId> {
        self.names.get(&name.to_lowercase()).copied()
    }

    fn interact(&mut self, from_name: &str, to_name: &str, speak: bool) {
        let (Some(fid), Some(tid)) = (self.resolve(from_name), self.resolve(to_name)) else {
            println!("Unknown agent in pair: '{}' / '{}'", from_name, to_name);
            return;
        };
        if fid == tid {
            println!("An agent cannot message itself.");
            return;
        }

        let (utterance, payload) = {
            let s = &self.agents[&fid];
            (
                if speak {
                    Some(s.generate_utterance(to_name))
                } else {
                    None
                },
                s.pack_own_states(),
            )
        };

        if let Some(ref u) = utterance {
            println!("{}", u);
        }
        let msg = Message {
            from: fid,
            to: tid,
            payload,
            utterance,
            ttl: 8,
        };
        self.agents.get_mut(&tid).unwrap().on_message(msg);
    }
}

fn main() {
    let mut inn = Inn::new();

    let mut bramble = Npc::new(0, "Bramble", Personality::Innkeeper);
    let mut lyra = Npc::new(1, "Lyra", Personality::Bard);
    let mut gruff = Npc::new(2, "Gruff", Personality::Mercenary);

    bramble.set_trust(AgentId(1), 0.80);
    bramble.set_trust(AgentId(2), 0.25);
    lyra.set_trust(AgentId(0), 0.75);
    lyra.set_trust(AgentId(2), 0.60);
    gruff.set_trust(AgentId(0), 0.35);
    gruff.set_trust(AgentId(1), 0.90);

    inn.add(bramble);
    inn.add(lyra);
    inn.add(gruff);

    println!("╔════════════════════════════════════════════════════╗");
    println!("║            THE CROSSROADS INN                     ║");
    println!("╠════════════════════════════════════════════════════╣");
    println!("║  bramble (id:0) — innkeeper                       ║");
    println!("║  lyra    (id:1) — bard                            ║");
    println!("║  gruff   (id:2) — mercenary                       ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!("Type 'help' for commands.\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.trim().splitn(5, ' ').collect();
        match parts.as_slice() {
            ["help"] | ["h"] => {
                println!("  status <n>                          — states + beliefs");
                println!("  set <n> health|hunger <0-100>       — set numeric state");
                println!("  set <n> gold <n>                    — set gold");
                println!("  set <n> mood joyful|neutral|wary|furious");
                println!("  tell  <from> <to>                   — share states silently");
                println!("  speak <from> <to>                   — speak + share states");
                println!("  trust <who> <toward> <0.0-1.0>      — set trust level");
                println!("  outcome <who> <toward> true|false   — claim confirmed/refuted");
                println!("  decay <n>                           — fade memories");
                println!("  quit");
            }
            ["quit"] | ["q"] | ["exit"] => break,

            ["status", n] => match inn.resolve(n) {
                Some(id) => inn.agents[&id].print_status(),
                None => println!("Unknown: '{}'", n),
            },

            ["set", n, "health", v] => match (inn.resolve(n), v.parse::<f32>()) {
                (Some(id), Ok(v)) => {
                    let v = v.clamp(0.0, 100.0);
                    inn.agents.get_mut(&id).unwrap().states.insert(Health(v));
                    if v < 40.0 {
                        println!("  [{}] is now injured.", n);
                    }
                    println!("Done.");
                }
                _ => println!("Usage: set <n> health <0-100>"),
            },
            ["set", n, "hunger", v] => match (inn.resolve(n), v.parse::<f32>()) {
                (Some(id), Ok(v)) => {
                    let v = v.clamp(0.0, 100.0);
                    inn.agents.get_mut(&id).unwrap().states.insert(Hunger(v));
                    if v > 80.0 {
                        println!("  [{}] is now starving.", n);
                    }
                    println!("Done.");
                }
                _ => println!("Usage: set <n> hunger <0-100>"),
            },
            ["set", n, "gold", v] => match (inn.resolve(n), v.parse::<u32>()) {
                (Some(id), Ok(v)) => {
                    inn.agents.get_mut(&id).unwrap().states.insert(Gold(v));
                    println!("Done.");
                }
                _ => println!("Usage: set <n> gold <n>"),
            },
            ["set", n, "mood", v] => {
                let mood = match *v {
                    "joyful" => Some(Mood::Joyful),
                    "neutral" => Some(Mood::Neutral),
                    "wary" => Some(Mood::Wary),
                    "furious" => Some(Mood::Furious),
                    _ => None,
                };
                match (inn.resolve(n), mood) {
                    (Some(id), Some(m)) => {
                        inn.agents.get_mut(&id).unwrap().states.insert(m);
                        println!("Done.");
                    }
                    _ => println!("Usage: set <n> mood joyful|neutral|wary|furious"),
                }
            }

            ["tell", f, t] => inn.interact(f, t, false),
            ["speak", f, t] => inn.interact(f, t, true),

            ["trust", who, toward, v] => {
                match (inn.resolve(who), inn.resolve(toward), v.parse::<f32>()) {
                    (Some(wid), Some(tid), Ok(v)) => {
                        inn.agents.get_mut(&wid).unwrap().set_trust(tid, v);
                        println!(
                            "{} now trusts {} at {:.0}%.",
                            who,
                            toward,
                            v.clamp(0.0, 1.0) * 100.0
                        );
                    }
                    _ => println!("Usage: trust <who> <toward> <0.0-1.0>"),
                }
            }

            ["outcome", who, toward, result] => {
                let confirmed = matches!(*result, "true" | "yes");
                match (inn.resolve(who), inn.resolve(toward)) {
                    (Some(wid), Some(tid)) => inn
                        .agents
                        .get_mut(&wid)
                        .unwrap()
                        .observe_outcome(tid, confirmed),
                    _ => println!("Usage: outcome <who> <toward> true|false"),
                }
            }

            ["decay", n] => match inn.resolve(n) {
                Some(id) => inn.agents.get_mut(&id).unwrap().decay(),
                None => println!("Unknown: '{}'", n),
            },

            _ => println!("Unknown command. Type 'help'."),
        }
    }

    println!("\nThe inn grows quiet.");
}
