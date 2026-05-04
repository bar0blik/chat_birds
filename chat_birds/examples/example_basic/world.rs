use std::collections::HashMap;

use chat_birds::{Agent, AgentId, DefaultCodec, MessageCodec, World};

pub struct MyWorld {
    agent_map: HashMap<AgentId, Box<dyn Agent>>,
}

impl MyWorld {
    pub fn new() -> Self {
        Self {
            agent_map: HashMap::new(),
        }
    }
}

impl World for MyWorld {
    fn codec(&self) -> Option<impl MessageCodec> {
        Some(DefaultCodec)
    }

    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>> {
        &self.agent_map
    }

    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>> {
        &mut self.agent_map
    }
}
