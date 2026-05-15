use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;

use chat_birds::{Agent, AgentId, DefaultCodec, Message, MessageCodec, State, World, impl_state};

mod agent;
mod commands;

use agent::*;
use commands::{SubCommand, parse_command};

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}

enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone, Copy, PartialEq)]
enum FocusedBlock {
    None,
    Agents,
    Search,
    Beliefs,
    Chat,
}

impl FocusedBlock {
    fn next(self) -> Self {
        match self {
            FocusedBlock::None => FocusedBlock::Agents,
            FocusedBlock::Agents => FocusedBlock::Search,
            FocusedBlock::Search => FocusedBlock::Beliefs,
            FocusedBlock::Beliefs => FocusedBlock::Chat,
            FocusedBlock::Chat => FocusedBlock::None,
        }
    }

    fn prev(self) -> Self {
        match self {
            FocusedBlock::None => FocusedBlock::Chat,
            FocusedBlock::Agents => FocusedBlock::None,
            FocusedBlock::Search => FocusedBlock::Agents,
            FocusedBlock::Beliefs => FocusedBlock::Search,
            FocusedBlock::Chat => FocusedBlock::Beliefs,
        }
    }
}

#[derive(Clone)]
struct Name(String);

impl_state!(Name);

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<(Option<Message>, Option<String>)>,
    /// Scroll offset for the chat messages
    messages_scroll: u16,
    /// Cached message viewport height
    messages_view_height: u16,
    /// Cached total message lines
    messages_total_lines: u16,
    /// Currently focused block
    focused_block: FocusedBlock,
    /// Currently active (inside) block
    active_block: Option<FocusedBlock>,
    /// All created agents
    agents: HashMap<AgentId, Box<dyn Agent>>,
    /// Agent ids in display/cycle order
    agent_order: Vec<AgentId>,
    /// Current selected agent
    selected_agent: Option<AgentId>,
}

impl World for App {
    fn codec(&self) -> Option<impl MessageCodec> {
        Some(DefaultCodec)
    }

    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>> {
        &self.agents
    }

    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>> {
        &mut self.agents
    }
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            messages_scroll: 0,
            messages_view_height: 0,
            messages_total_lines: 0,
            character_index: 0,
            focused_block: FocusedBlock::Chat,
            active_block: None,
            agents: HashMap::new(),
            agent_order: Vec::new(),
            selected_agent: None,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    /// Handles message submitting and command dispatch
    fn submit_message(&mut self) {
        if self.input.starts_with('$') {
            // Try to parse as a command
            let cmd_str = &self.input[1..]; // Remove the '$' prefix
            match parse_command(cmd_str) {
                Ok(cmd) => match cmd.cmd {
                    SubCommand::Agent(agent_cmd) => {
                        if let Some(name) = agent_cmd.new {
                            match self.create_agent(name) {
                                Ok((id, name)) => {
                                    let msg = format!("Created agent '{}' (id {})", name, id.0);
                                    self.messages.push((None, Some(msg)));
                                }
                                Err(err) => {
                                    self.messages.push((None, Some(err)));
                                }
                            }
                        }

                        if let Some(agent) = agent_cmd.kill {
                            match self.kill_agent(&agent) {
                                Ok((id, name)) => {
                                    let msg = format!("Killed agent '{}' (id {})", name, id.0);
                                    self.messages.push((None, Some(msg)));
                                }
                                Err(err) => {
                                    self.messages.push((None, Some(err)));
                                }
                            }
                        }

                        if let Some(agent) = agent_cmd.select.or(agent_cmd.select_default) {
                            match self.select_agent(&agent) {
                                Ok(()) => {}
                                Err(err) => {
                                    self.messages.push((None, Some(err)));
                                }
                            }
                        }
                    }
                    SubCommand::Channel(_) => {
                        self.messages
                            .push((None, Some("channel command parsed".into())));
                    }
                    SubCommand::Reply(_) => {
                        self.messages
                            .push((None, Some("reply command parsed".into())));
                    }
                },
                Err(e) => {
                    // Parse failed, show error message
                    let error_msg = format!("{}", e);
                    self.messages.push((None, Some(error_msg)));
                }
            }
        } else {
            // Regular message (original behavior)
            let message = (
                self.codec()
                    .unwrap()
                    .decode(&self.input, AgentId(0), AgentId(0)),
                Some(self.input.clone()),
            );
            self.messages.push(message);
        }
        self.input.clear();
        self.messages_scroll = u16::MAX;
        self.reset_cursor();
    }

    fn create_agent(&mut self, name: String) -> Result<(AgentId, String), String> {
        let id = self.next_agent_id().ok_or("No available agent IDs")?;
        let mut agent = Bird::new(id.0);
        agent.states_mut().insert(Name(name.clone()));
        self.agents.insert(id, Box::new(agent));
        self.agent_order.push(id);
        Ok((id, name))
    }

    fn select_agent(&mut self, agent: &str) -> Result<(), String> {
        let target = agent.trim();
        if target.is_empty() {
            return Err("Missing agent name or id".into());
        }

        if let Ok(id) = target.parse::<u16>() {
            let agent_id = AgentId(id);
            if self.agents.contains_key(&agent_id) {
                self.selected_agent = Some(agent_id);
                return Ok(());
            }
        }

        if let Some((agent_id, _)) = self.agents.iter().find(|(_, a)| {
            a.states()
                .get::<Name>()
                .map(|n| n.0.as_str() == target)
                .unwrap_or(false)
        }) {
            self.selected_agent = Some(*agent_id);
            return Ok(());
        }

        Err("Unknown target".into())
    }

    fn kill_agent(&mut self, agent: &str) -> Result<(AgentId, String), String> {
        let target = agent.trim();

        // Kill selected agent if no agent specified
        if target.is_empty() {
            if let Some(agent_id) = self.selected_agent {
                if agent_id == AgentId(0) {
                    return Err("Cannot kill user agent".into());
                }
                let name = self
                    .agents
                    .remove(&agent_id)
                    .unwrap()
                    .states()
                    .get::<Name>()
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("agent-{}", agent_id.0));
                self.agent_order.retain(|e| e != &agent_id);
                return Ok((agent_id, name));
            }
            return Err("Missing agent name or id".into());
        }

        // Kill agent from id
        if let Ok(id) = target.parse::<u16>() {
            let agent_id = AgentId(id);
            if agent_id == AgentId(0) {
                return Err("Cannot kill user agent".into());
            }
            if self.agents.contains_key(&agent_id) {
                let name = self
                    .agents
                    .remove(&agent_id)
                    .unwrap()
                    .states()
                    .get::<Name>()
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("agent-{}", id));
                self.agent_order.retain(|e| e != &agent_id);
                return Ok((agent_id, name));
            }
        }

        // Kill agent from name
        if let Some(agent_id) = self
            .agents
            .iter()
            .find(|(_, a)| {
                a.states()
                    .get::<Name>()
                    .map(|n| n.0.as_str() == target)
                    .unwrap_or(false)
            })
            .map(|(agent_id, _)| *agent_id)
        {
            if agent_id == AgentId(0) {
                return Err("Cannot kill user agent".into());
            }
            self.agents.remove(&agent_id);
            self.agent_order.retain(|e| e != &agent_id);
            return Ok((agent_id, target.into()));
        }

        Err("Unknown target".into())
    }

    fn select_next_agent(&mut self) {
        if self.agent_order.is_empty() {
            self.selected_agent = None;
            return;
        }

        let next_id = match self.selected_agent {
            Some(current) => self
                .agent_order
                .iter()
                .position(|id| *id == current)
                .map(|idx| self.agent_order[(idx + 1) % self.agent_order.len()])
                .unwrap_or(self.agent_order[0]),
            None => self.agent_order[0],
        };

        self.selected_agent = Some(next_id);
    }

    fn select_prev_agent(&mut self) {
        if self.agent_order.is_empty() {
            self.selected_agent = None;
            return;
        }

        let prev_id = match self.selected_agent {
            Some(current) => self
                .agent_order
                .iter()
                .position(|id| *id == current)
                .map(|idx| {
                    self.agent_order[(idx + self.agent_order.len() - 1) % self.agent_order.len()]
                })
                .unwrap_or(self.agent_order[0]),
            None => self.agent_order[0],
        };

        self.selected_agent = Some(prev_id);
    }

    fn next_agent_id(&self) -> Option<AgentId> {
        let max_id = self.agents.keys().map(|id| id.0).max().unwrap_or(0);
        max_id.checked_add(1).map(AgentId)
    }

    fn update_messages_metrics(&mut self, total_lines: u16, view_height: u16) {
        self.messages_total_lines = total_lines;
        self.messages_view_height = view_height;
        let max_scroll = total_lines.saturating_sub(view_height);
        if self.messages_scroll > max_scroll {
            self.messages_scroll = max_scroll;
        }
    }

    fn count_wrapped_lines(&self, lines: &[Line], width: u16) -> u16 {
        let width = width.max(1) as usize;
        let mut total: usize = 0;
        for line in lines {
            let line_width = line.width().max(1);
            total += (line_width + width - 1) / width;
        }
        total as u16
    }

    fn scroll_messages_by(&mut self, delta: i16) {
        let max_scroll = self
            .messages_total_lines
            .saturating_sub(self.messages_view_height) as i16;
        let next = (self.messages_scroll as i16 + delta).clamp(0, max_scroll);
        self.messages_scroll = next as u16;
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let user = User::new();
        let user_id = user.id();
        self.agents.insert(user_id, Box::new(user));
        self.agent_order.push(user_id);
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(key) = event::read()?.as_key_press_event() {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') if self.active_block.is_none() => return Ok(()),
                        // Shift+Tab cycles selected agent backwards when inside the Agents block
                        KeyCode::BackTab if self.active_block == Some(FocusedBlock::Agents) => {
                            self.select_prev_agent();
                        }
                        // Tab cycles selected agent when inside the Agents block
                        KeyCode::Tab if self.active_block == Some(FocusedBlock::Agents) => {
                            self.select_next_agent();
                        }
                        // Shift+Tab cycles focus backwards when not inside a block
                        KeyCode::BackTab if self.active_block.is_none() => {
                            self.focused_block = self.focused_block.prev();
                        }
                        // Tab cycles focus only when not inside a block
                        KeyCode::Tab if self.active_block.is_none() => {
                            self.focused_block = self.focused_block.next();
                        }
                        // Enter enters the currently focused block (becomes active)
                        KeyCode::Enter
                            if self.active_block.is_none()
                                && self.focused_block != FocusedBlock::None =>
                        {
                            self.active_block = Some(self.focused_block);
                            if self.focused_block == FocusedBlock::Chat {
                                self.input_mode = InputMode::Editing;
                            }
                        }
                        // Esc leaves an active block, or clears focus when not inside
                        KeyCode::Esc if self.active_block.is_some() => {
                            self.active_block = None;
                            self.input_mode = InputMode::Normal;
                        }
                        KeyCode::Esc if self.active_block.is_none() => {
                            self.focused_block = FocusedBlock::None;
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        // Scroll chat history when inside the Chat block
                        KeyCode::Up if self.active_block == Some(FocusedBlock::Chat) => {
                            self.scroll_messages_by(-1);
                        }
                        KeyCode::Down if self.active_block == Some(FocusedBlock::Chat) => {
                            self.scroll_messages_by(1);
                        }
                        KeyCode::PageUp if self.active_block == Some(FocusedBlock::Chat) => {
                            let step = self.messages_view_height.max(1) as i16;
                            self.scroll_messages_by(-step);
                        }
                        KeyCode::PageDown if self.active_block == Some(FocusedBlock::Chat) => {
                            let step = self.messages_view_height.max(1) as i16;
                            self.scroll_messages_by(step);
                        }
                        // When editing (inside chat) Enter sends message
                        KeyCode::Enter => {
                            self.submit_message();
                        }
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        // Esc leaves the active block and stops editing
                        KeyCode::Esc => {
                            self.input_mode = InputMode::Normal;
                            self.active_block = None;
                        }
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        use Constraint::*;
        // Vertical layout for help bar
        let vertical = Layout::vertical([Fill(1), Length(1)]);
        let [main_area, help_area] = vertical.areas(frame.area());
        let help_message = if let Some(active) = self.active_block {
            match active {
                FocusedBlock::Chat => Paragraph::new(Text::from(Line::from(vec![
                    "Type to edit, ".into(),
                    "Enter".bold(),
                    " to send, ".into(),
                    "Esc".bold(),
                    " to leave".into(),
                ]))),
                FocusedBlock::None => Paragraph::new(""),
                _ => Paragraph::new(Text::from(Line::from(vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to leave".into(),
                ]))),
            }
        } else {
            match self.focused_block {
                FocusedBlock::None => Paragraph::new(Text::from(Line::from(vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "Tab".bold(),
                    " to cycle blocks".into(),
                ]))),
                _ => Paragraph::new(Text::from(Line::from(vec![
                    "Press ".into(),
                    "Enter".bold(),
                    " to enter, ".into(),
                    "Tab".bold(),
                    " to cycle, ".into(),
                    "q".bold(),
                    " to quit".into(),
                ]))),
            }
        };
        frame.render_widget(help_message, help_area);

        // Main layout: left sidebar and main content area
        let horizontal = Layout::horizontal([Ratio(1, 5), Fill(1)]);
        let [left_area, right_area] = horizontal.areas(main_area);

        // Left sidebar: agents and search
        let left = Layout::vertical([Ratio(1, 2); 2]);
        let [agents_area, search_area] = left.areas(left_area);

        // Right side: beliefs and chat
        let right = Layout::vertical([Fill(1), Ratio(1, 3)]);
        let [beliefs_area, chat_area] = right.areas(right_area);

        // Agents panel
        let agents_block = if self.active_block == Some(FocusedBlock::Agents) {
            // inside: colored border, not thick
            Block::bordered()
                .title("Agents".black().bold().on_yellow())
                .yellow()
        } else if self.focused_block == FocusedBlock::Agents {
            // focused (but not inside): thick border
            Block::bordered()
                .border_type(BorderType::Thick)
                .title("Agents".black().bold().on_yellow())
                .yellow()
        } else {
            Block::bordered().title("Agents".yellow())
        };
        let agents_list_area = agents_block.inner(agents_area);
        frame.render_widget(agents_block, agents_area);

        let agents: Vec<ListItem> = self
            .agent_order
            .iter()
            .filter_map(|id| self.agents.get(id).map(|agent| (id, agent)))
            .map(|(id, agent)| {
                let name = agent
                    .states()
                    .get::<Name>()
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("agent-{}", id.0));
                // TODO: add arrows to indicate channel members
                let content = Line::from(Span::raw(format!("{}: {}", id.0, name)));
                if let Some(selected) = self.selected_agent
                    && &selected == id
                {
                    ListItem::new(content).black().bold().on_yellow()
                } else {
                    ListItem::new(content)
                }
            })
            .collect();
        let agents_widget = List::new(agents);
        frame.render_widget(agents_widget, agents_list_area);

        // Search panel
        let search_block = if self.active_block == Some(FocusedBlock::Search) {
            Block::bordered()
                .title("Search".black().bold().on_red())
                .red()
        } else if self.focused_block == FocusedBlock::Search {
            Block::bordered()
                .border_type(BorderType::Thick)
                .title("Search".black().bold().on_red())
                .red()
        } else {
            Block::bordered().title("Search".red())
        };
        frame.render_widget(search_block, search_area);

        // Beliefs panel
        let beliefs_block = if self.active_block == Some(FocusedBlock::Beliefs) {
            Block::bordered()
                .title("Beliefs".black().bold().on_green())
                .green()
        } else if self.focused_block == FocusedBlock::Beliefs {
            Block::bordered()
                .border_type(BorderType::Thick)
                .title("Beliefs".black().bold().on_green())
                .green()
        } else {
            Block::bordered().title("Beliefs".green())
        };
        frame.render_widget(beliefs_block, beliefs_area);

        // Chat panel with outer border
        let chat_block = if self.active_block == Some(FocusedBlock::Chat) {
            // inside chat: colored border
            Block::bordered()
                .title("Chat".black().bold().on_blue())
                .blue()
        } else if self.focused_block == FocusedBlock::Chat {
            // focused (but not inside): thick border
            Block::bordered()
                .border_type(BorderType::Thick)
                .title("Chat".black().bold().on_blue())
                .blue()
        } else {
            Block::bordered().title("Chat".blue())
        };
        frame.render_widget(chat_block, chat_area);

        // Inner layout for chat content: messages above, input at bottom
        let inner_area = chat_area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });
        let chat_layout = Layout::vertical([Fill(1), Length(1)]);
        let [messages_area, input_area] = chat_layout.areas(inner_area);

        // Messages list without additional border
        let messages_lines: Vec<Line> = self
            .messages
            .iter()
            .map(|m| {
                let mut from = AgentId(0);
                if let Some(msg) = &m.0 {
                    from = msg.from;
                }
                Line::from(Span::raw(format!(
                    "{}: {}",
                    self.agents
                        .get(&from)
                        .unwrap()
                        .states()
                        .get::<Name>()
                        .unwrap_or(&Name(format!("agent-{}", from.0)))
                        .0,
                    // avoid moving out of the borrowed tuple `m` by taking a reference
                    (&m.1).as_deref().unwrap_or("")
                )))
            })
            .collect();

        let total_lines = self.count_wrapped_lines(&messages_lines, messages_area.width);
        self.update_messages_metrics(total_lines, messages_area.height);
        let messages_text = Text::from(messages_lines);
        let messages_widget = Paragraph::new(messages_text)
            .wrap(Wrap { trim: false })
            .scroll((self.messages_scroll, 0));
        frame.render_widget(messages_widget, messages_area);

        if self.messages_total_lines > self.messages_view_height {
            let mut scrollbar_state = ScrollbarState::new(self.messages_total_lines as usize)
                .position(self.messages_scroll as usize)
                .viewport_content_length(self.messages_view_height as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            frame.render_stateful_widget(scrollbar, messages_area, &mut scrollbar_state);
        }

        // Input box without additional border
        let input =
            Paragraph::new(">>> ".to_owned() + self.input.as_str()).style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            });
        frame.render_widget(input, input_area);

        // Set cursor position when editing
        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Editing =>
            {
                #[expect(clippy::cast_possible_truncation)]
                frame.set_cursor_position(Position::new(
                    input_area.x + 4 + self.character_index as u16,
                    input_area.y,
                ))
            }
        }
    }
}
