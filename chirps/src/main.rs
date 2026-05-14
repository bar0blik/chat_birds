use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
    /// Currently focused block
    focused_block: FocusedBlock,
    /// Currently active (inside) block
    active_block: Option<FocusedBlock>,
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
}

impl App {
    const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            character_index: 0,
            focused_block: FocusedBlock::Chat,
            active_block: None,
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

    fn submit_message(&mut self) {
        self.messages.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(key) = event::read()?.as_key_press_event() {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') if self.active_block.is_none() => return Ok(()),
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

    fn render(&self, frame: &mut Frame) {
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
        frame.render_widget(agents_block, agents_area);

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
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let content = Line::from(Span::raw(format!("{i}: {m}")));
                ListItem::new(content)
            })
            .collect();
        let messages_widget = List::new(messages);
        frame.render_widget(messages_widget, messages_area);

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
