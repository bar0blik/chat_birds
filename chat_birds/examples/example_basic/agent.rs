use chat_birds::{Agent, AgentId, BeliefStore, Message, SourceMap, StateMap};

pub struct MyAgent {
    id: AgentId,
    states: StateMap,
    beliefs: BeliefStore,
    sources: SourceMap,
}

impl MyAgent {
    pub fn new(id: u16) -> Self {
        Self {
            id: AgentId(id),
            states: StateMap::new(),
            beliefs: BeliefStore::new(),
            sources: SourceMap::new(),
        }
    }
}

impl Agent for MyAgent {
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

    fn source_map(&self) -> &SourceMap {
        &self.sources
    }

    fn source_map_mut(&mut self) -> &mut SourceMap {
        &mut self.sources
    }

    fn on_message(&mut self, _msg: Message) -> Vec<Message> {
        Vec::new()
    }
}
