use chat_birds::{
    register_builtin_state_reprs, register_state_repr, Agent, Belief, BeliefSource, BeliefStore,
    Clock, DefaultCodec, Message, MessageCodec, Nature, Probability, SubjectBeliefs, Temporal,
    TenseTime,
};

#[path = "example_basic/agent.rs"]
mod agent;
#[path = "example_basic/state/mod.rs"]
mod state;
#[path = "example_basic/world.rs"]
mod world;

use agent::MyAgent;
use state::{Color, ColorState, Feeling, FeelingState, RunningState};
use world::MyWorld;

fn main() {
    register_builtin_state_reprs();
    register_state_repr!(ColorState);
    register_state_repr!(FeelingState);
    register_state_repr!(RunningState);

    let _world = MyWorld::new();
    let agent1 = MyAgent::new(1);
    let _agent2 = MyAgent::new(2);
    let clock = Clock::system();
    let mut payload = BeliefStore::new();
    let mut beliefs = SubjectBeliefs::new();
    let nature_belief = Belief {
        state: Box::from(Nature("an agent".into())),
        certainty: 255,
        probability: Probability::Always,
        source: BeliefSource::Myself,
        temporal: Temporal::Always,
    };
    let color_belief = Belief {
        state: Box::from(ColorState::new(Color::Blue)),
        certainty: 255,
        probability: Probability::Always,
        source: BeliefSource::Myself,
        temporal: Temporal::Always,
    };
    let feeling_belief = Belief {
        state: Box::from(FeelingState::new(Feeling::Fine)),
        certainty: 200,
        probability: Probability::Always,
        source: BeliefSource::Myself,
        temporal: Temporal::Tense(TenseTime::Present),
    };
    let running_belief = Belief {
        state: Box::from(RunningState),
        certainty: 220,
        probability: Probability::Always,
        source: BeliefSource::Myself,
        temporal: Temporal::Period {
            start: Box::from(Temporal::Tense(TenseTime::Past)),
            end: Box::from(Temporal::Tense(TenseTime::Present)),
        },
    };
    beliefs.map.insert::<Nature>(nature_belief);
    beliefs.map.insert::<ColorState>(color_belief);
    beliefs.map.insert::<FeelingState>(feeling_belief);
    beliefs.map.insert::<RunningState>(running_belief);
    payload.insert(&agent1.id(), beliefs);
    let msg = Message {
        from: agent1.id(),
        to: agent1.id(),
        payload,
    };
    let s = DefaultCodec.encode(&msg, &clock);
    println!("{}", s);
}
