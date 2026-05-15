use crate::Name;
use chat_birds::{Agent, AgentId, BeliefStore, Message, SourceMap, StateMap};

pub struct Bird {
    id: AgentId,
    states: StateMap,
    beliefs: BeliefStore,
    sources: SourceMap,
}

impl Bird {
    pub fn new(id: u16) -> Self {
        Self {
            id: AgentId(id),
            states: StateMap::new(),
            beliefs: BeliefStore::new(),
            sources: SourceMap::new(),
        }
    }
}

impl Agent for Bird {
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

    fn on_message(&mut self, msg: Message) {}

    fn reply(&mut self, id: AgentId) -> Message {
        Message {
            from: self.id(),
            to: id,
            payload: BeliefStore::new(),
        }
    }
}

pub struct User {
    id: AgentId,
    _states: StateMap,
    _beliefs: BeliefStore,
    _sources: SourceMap,
}

impl User {
    pub fn new() -> Self {
        let mut states = StateMap::new();
        states.insert(Name("user".into()));
        Self {
            id: AgentId(0),
            _states: states,
            _beliefs: BeliefStore::new(),
            _sources: SourceMap::new(),
        }
    }
}

impl Agent for User {
    fn id(&self) -> AgentId {
        self.id
    }

    fn states(&self) -> &StateMap {
        &self._states
    }

    fn states_mut(&mut self) -> &mut StateMap {
        &mut self._states
    }

    fn beliefs(&self) -> &BeliefStore {
        &self._beliefs
    }

    fn beliefs_mut(&mut self) -> &mut BeliefStore {
        &mut self._beliefs
    }

    fn source_map(&self) -> &SourceMap {
        &self._sources
    }

    fn source_map_mut(&mut self) -> &mut SourceMap {
        &mut self._sources
    }

    fn on_message(&mut self, msg: Message) {}

    fn reply(&mut self, id: AgentId) -> Message {
        Message {
            from: self.id(),
            to: id,
            payload: BeliefStore::new(),
        }
    }
}
