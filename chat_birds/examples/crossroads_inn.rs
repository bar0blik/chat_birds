use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use chat_birds::*;
// ── States ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Mood {
    Joyful,
    Neutral,
    Wary,
    Furious,
}

#[derive(Clone, Debug)]
struct Health(pub f32);
#[derive(Clone, Debug)]
struct Hunger(pub f32);
#[derive(Clone, Debug)]
struct Gold(pub u32);

impl_state!(Mood);
impl_state!(Health);
impl_state!(Hunger);
impl_state!(Gold);

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

// ── Personality ───────────────────────────────────────────────────────────────

enum Personality {
    Innkeeper,
    Bard,
    Mercenary,
}

// ── Npc ───────────────────────────────────────────────────────────────────────

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

    fn set_trust(&mut self, other: AgentId, trust: f32) {
        self.trust.insert(other, trust.clamp(0.0, 1.0));
    }

    /// Packs own StateMap into a BeliefStore to send as a message payload.
    fn pack_own_states(&self) -> BeliefStore {
        let mut store = BeliefStore::new();
        let bmap = store.get_or_insert(&self.name.as_str());
        for (tid, state) in &self.states.0 {
            bmap.0.entry(*tid).or_default().push(BeliefEntry {
                state: state.clone_box(),
                certainty: 1.0,
                probability: Probability::Always,
                source: BeliefSource::Myself,
                temporal: Temporal::Tense(Tense::Present),
            });
        }
        store
    }

    fn describe_entry(entry: &BeliefEntry) -> String {
        let s = &entry.state;
        let cert = entry.certainty * 100.0;
        if let Some(h) = s.as_any().downcast_ref::<Health>() {
            format!("health={:.0} ({:.0}%)", h.0, cert)
        } else if let Some(h) = s.as_any().downcast_ref::<Hunger>() {
            format!("hunger={:.0} ({:.0}%)", h.0, cert)
        } else if let Some(m) = s.as_any().downcast_ref::<Mood>() {
            format!("mood={} ({:.0}%)", m, cert)
        } else if let Some(g) = s.as_any().downcast_ref::<Gold>() {
            format!("gold={} ({:.0}%)", g.0, cert)
        } else {
            "?".to_string()
        }
    }

    fn print_status(&self) {
        let health = self.states.get::<Health>().map(|h| h.0).unwrap_or(0.0);
        let hunger = self.states.get::<Hunger>().map(|h| h.0).unwrap_or(0.0);
        let mood = self.states.get::<Mood>().cloned().unwrap_or(Mood::Neutral);
        let gold = self.states.get::<Gold>().map(|g| g.0).unwrap_or(0);

        println!("┌─ {}", self.name);
        println!(
            "│  Health {:>5.0}  Hunger {:>5.0}  Mood {:>8}  Gold {:>4}",
            health, hunger, mood, gold
        );

        if !self.trust.is_empty() {
            let ts: Vec<String> = self
                .trust
                .iter()
                .map(|(id, t)| format!("agent{}={:.0}%", id.0, t * 100.0))
                .collect();
            println!("│  Trust: {}", ts.join(", "));
        }

        if self.beliefs.0.is_empty() {
            println!("│  Beliefs: (none)");
        } else {
            println!("│  Beliefs:");
            for (key, bmap) in &self.beliefs.0 {
                let parts: Vec<String> = bmap
                    .0
                    .values()
                    .filter_map(|entries| {
                        entries
                            .iter()
                            .max_by(|a, b| a.certainty.partial_cmp(&b.certainty).unwrap())
                    })
                    .map(Self::describe_entry)
                    .collect();
                if !parts.is_empty() {
                    println!("│    '{}': {}", key, parts.join(", "));
                }
            }
        }
        println!("└──────────────────────────────────────────────");
    }

    fn generate_utterance_to(&self, target: &str) -> String {
        let mood = self.states.get::<Mood>().cloned().unwrap_or(Mood::Neutral);
        let hunger = self.states.get::<Hunger>().map(|h| h.0).unwrap_or(0.0);
        let health = self.states.get::<Health>().map(|h| h.0).unwrap_or(100.0);

        match self.personality {
            Personality::Innkeeper => {
                if hunger > 70.0 {
                    format!(
                        "{}: \"Pardon me, {}, but I haven't eaten since dawn. Business after?\"",
                        self.name, target
                    )
                } else if health < 40.0 {
                    format!(
                        "{}: \"Not at my best today, {}. What do you need?\"",
                        self.name, target
                    )
                } else {
                    match mood {
                        Mood::Joyful => format!(
                            "{}: \"Welcome, {}! First round's on me tonight!\"",
                            self.name, target
                        ),
                        Mood::Wary => format!(
                            "{}: \"I'll be watching you, {}. Don't make me regret it.\"",
                            self.name, target
                        ),
                        Mood::Furious => format!(
                            "{}: \"You've got some nerve showing up here, {}.\"",
                            self.name, target
                        ),
                        Mood::Neutral => format!("{}: \"What'll it be, {}?\"", self.name, target),
                    }
                }
            }
            Personality::Bard => {
                if hunger > 70.0 {
                    format!(
                        "{}: \"Even muses must eat, dear {}. A ballad after supper?\"",
                        self.name, target
                    )
                } else {
                    match mood {
                        Mood::Joyful => format!(
                            "{}: \"{}! You arrive like a chorus after a long verse!\"",
                            self.name, target
                        ),
                        Mood::Furious => format!(
                            "{}: \"Not now, {}. Someone's stolen my best ballad and I intend to find them.\"",
                            self.name, target
                        ),
                        Mood::Wary => format!(
                            "{}: \"I've heard stories about you, {}. Interesting ones.\"",
                            self.name, target
                        ),
                        Mood::Neutral => format!(
                            "{}: \"Lovely to see you, {}. Sit, and I'll play something fitting.\"",
                            self.name, target
                        ),
                    }
                }
            }
            Personality::Mercenary => {
                if health < 40.0 {
                    format!(
                        "{}: \"Took some hits, {}. Still standing.\"",
                        self.name, target
                    )
                } else if hunger > 70.0 {
                    format!("{}: \"Hungry. Talk later, {}.\"", self.name, target)
                } else {
                    match mood {
                        Mood::Joyful => format!(
                            "{}: \"Good contract today. Buy you a drink, {}?\"",
                            self.name, target
                        ),
                        Mood::Furious => format!(
                            "{}: *cracks knuckles* \"You want trouble, {}?\"",
                            self.name, target
                        ),
                        Mood::Wary => {
                            format!("{}: \"Something's off about you, {}.\"", self.name, target)
                        }
                        Mood::Neutral => format!("{}: \"Speak, {}.\"", self.name, target),
                    }
                }
            }
        }
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
        for (_, bmap) in self.beliefs.0.iter_mut() {
            for (_, entries) in bmap.0.iter_mut() {
                for e in entries.iter_mut() {
                    e.certainty = (e.certainty - 0.15).max(0.0);
                }
                let before = entries.len();
                entries.retain(|e| e.certainty > 0.0);
                forgotten += before - entries.len();
            }
        }
        println!(
            "  [{}] memory fades. {} belief(s) forgotten.",
            self.name, forgotten
        );
    }

    fn resolve_belief(
        &self,
        _key: &str,
        from: AgentId,
        existing: Vec<BeliefEntry>,
        mut incoming: BeliefEntry,
    ) -> Vec<BeliefEntry> {
        let trust = self.trust.get(&from).copied().unwrap_or(0.5);
        incoming.certainty *= trust;

        // Replace any same-type entry; keep incoming regardless of certainty
        // (agent always records what it heard, scaled by trust).
        let incoming_tid = incoming.state.as_any().type_id();
        let mut result: Vec<BeliefEntry> = existing
            .into_iter()
            .filter(|e| e.state.as_any().type_id() != incoming_tid)
            .collect();
        result.push(incoming);
        result
    }
}

// ── Inn ───────────────────────────────────────────────────────────────────────

struct Inn {
    agents: HashMap<AgentId, Npc>,
    names: HashMap<String, AgentId>,
}

impl Inn {
    fn new() -> Self {
        Inn {
            agents: HashMap::new(),
            names: HashMap::new(),
        }
    }

    fn add(&mut self, npc: Npc) {
        self.names.insert(npc.name.to_lowercase(), npc.id);
        self.agents.insert(npc.id, npc);
    }

    fn resolve(&self, name: &str) -> Option<AgentId> {
        self.names.get(&name.to_lowercase()).copied()
    }

    fn interact(&mut self, from_name: &str, to_name: &str, with_speech: bool) {
        let from_id = match self.resolve(from_name) {
            Some(id) => id,
            None => {
                println!("Unknown agent: '{}'", from_name);
                return;
            }
        };
        let to_id = match self.resolve(to_name) {
            Some(id) => id,
            None => {
                println!("Unknown agent: '{}'", to_name);
                return;
            }
        };
        if from_id == to_id {
            println!("An agent cannot message itself.");
            return;
        }

        let (utterance, payload) = {
            let sender = &self.agents[&from_id];
            let u = if with_speech {
                Some(sender.generate_utterance_to(to_name))
            } else {
                None
            };
            (u, sender.pack_own_states())
        };

        if let Some(ref u) = utterance {
            println!("{}", u);
        }

        let msg = Message {
            from: from_id,
            to: to_id,
            payload,
            utterance,
            ttl: 8,
        };
        self.agents.get_mut(&to_id).unwrap().on_message(msg);
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut inn = Inn::new();

    let mut bramble = Npc::new(0, "Bramble", Personality::Innkeeper);
    let mut lyra = Npc::new(1, "Lyra", Personality::Bard);
    let mut gruff = Npc::new(2, "Gruff", Personality::Mercenary);

    // Bramble trusts Lyra, is wary of Gruff
    bramble.set_trust(AgentId(1), 0.75);
    bramble.set_trust(AgentId(2), 0.25);
    // Lyra trusts everyone
    lyra.set_trust(AgentId(0), 0.80);
    lyra.set_trust(AgentId(2), 0.60);
    // Gruff is suspicious of Bramble but oddly trusts the bard
    gruff.set_trust(AgentId(0), 0.35);
    gruff.set_trust(AgentId(1), 0.90);

    inn.add(bramble);
    inn.add(lyra);
    inn.add(gruff);

    println!("╔══════════════════════════════════════════════╗");
    println!("║           THE CROSSROADS INN                ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  bramble (id:0) — innkeeper                 ║");
    println!("║  lyra    (id:1) — bard                      ║");
    println!("║  gruff   (id:2) — mercenary                 ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("Type 'help' for commands.\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.trim().is_empty() {
            if line.is_empty() {
                break;
            } // EOF
            continue;
        }

        let parts: Vec<&str> = line.trim().splitn(4, ' ').collect();
        match parts.as_slice() {
            ["help"] | ["h"] => {
                println!("  status <name>                    — show states + beliefs");
                println!("  set    <name> health <0-100>     — set health");
                println!("  set    <name> hunger <0-100>     — set hunger");
                println!("  set    <name> gold   <n>         — set gold");
                println!("  set    <name> mood   joyful|neutral|wary|furious");
                println!("  tell   <from> <to>               — share states silently");
                println!("  speak  <from> <to>               — speak + share states");
                println!("  trust  <who> <toward> <0.0-1.0>  — set trust level");
                println!("  decay  <name>                    — fade memories (-15% certainty)");
                println!("  quit");
            }

            ["quit"] | ["q"] | ["exit"] => break,

            ["status", name] => match inn.resolve(name) {
                Some(id) => inn.agents[&id].print_status(),
                None => println!("Unknown: '{}'", name),
            },

            ["set", name, "health", val] => match (inn.resolve(name), val.parse::<f32>()) {
                (Some(id), Ok(v)) => {
                    inn.agents
                        .get_mut(&id)
                        .unwrap()
                        .states
                        .insert(Health(v.clamp(0.0, 100.0)));
                    println!("Done.");
                }
                _ => println!("Usage: set <name> health <0-100>"),
            },
            ["set", name, "hunger", val] => match (inn.resolve(name), val.parse::<f32>()) {
                (Some(id), Ok(v)) => {
                    inn.agents
                        .get_mut(&id)
                        .unwrap()
                        .states
                        .insert(Hunger(v.clamp(0.0, 100.0)));
                    println!("Done.");
                }
                _ => println!("Usage: set <name> hunger <0-100>"),
            },
            ["set", name, "gold", val] => match (inn.resolve(name), val.parse::<u32>()) {
                (Some(id), Ok(v)) => {
                    inn.agents.get_mut(&id).unwrap().states.insert(Gold(v));
                    println!("Done.");
                }
                _ => println!("Usage: set <name> gold <n>"),
            },
            ["set", name, "mood", val] => {
                let mood = match *val {
                    "joyful" => Some(Mood::Joyful),
                    "neutral" => Some(Mood::Neutral),
                    "wary" => Some(Mood::Wary),
                    "furious" => Some(Mood::Furious),
                    _ => None,
                };
                match (inn.resolve(name), mood) {
                    (Some(id), Some(m)) => {
                        inn.agents.get_mut(&id).unwrap().states.insert(m);
                        println!("Done.");
                    }
                    _ => println!("Usage: set <name> mood joyful|neutral|wary|furious"),
                }
            }

            ["tell", from, to] => inn.interact(from, to, false),
            ["speak", from, to] => inn.interact(from, to, true),

            ["trust", who, toward, val] => {
                match (inn.resolve(who), inn.resolve(toward), val.parse::<f32>()) {
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

            ["decay", name] => match inn.resolve(name) {
                Some(id) => inn.agents.get_mut(&id).unwrap().decay(),
                None => println!("Unknown: '{}'", name),
            },

            _ => println!("Unknown command. Type 'help'."),
        }
    }

    println!("\nThe inn grows quiet.");
}
