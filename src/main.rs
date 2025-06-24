use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::game::{CursorDirection, Game, Tile, TileContent, TileMistake, TileMode};
mod game;

fn main() -> Result<()> {
    let [width, height, mines]: [usize; 3] = std::env::args()
        .skip(1)
        .map(|v| v.parse::<usize>().context("parameters should be usize"))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| eyre!("should only give two size parameters and one mine count parameter"))?;

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new((width, height), mines).run(terminal);
    ratatui::restore();
    result
}

struct App {
    running: bool,
    game: Game,
}

trait RenderTile {
    fn render_tile(&self, is_selected: bool) -> ratatui::text::Span<'static>;
}

impl RenderTile for Tile {
    fn render_tile(&self, is_selected: bool) -> ratatui::text::Span<'static> {
        let res = match (&self.mode, &self.content) {
            (TileMode::Hidden, _) => "-".white(),
            (TileMode::Flagged, _) => "î".red(),
            (TileMode::Revealed, TileContent::Mine) => "*".black(),
            (TileMode::Revealed, TileContent::Field(n)) => match n {
                0 => " ".white(),
                1 => "1".magenta(),
                2 => "2".green(),
                3 => "3".yellow(),
                4 => "4".cyan(),
                5 => "5".magenta(),
                6 => "6".green(),
                7 => "7".yellow(),
                8 => "8".cyan(),
                _ => "?".white(),
            },
            (_, TileContent::Mistake(TileMistake::TrippedMine)) => "*".white().on_red(),
            (_, TileContent::Mistake(TileMistake::FlaggedField(n))) => {
                n.to_string().white().on_red()
            }
        };
        if is_selected {
            res.underlined()
        } else {
            res.not_underlined()
        }
    }
}

impl App {
    pub fn new(size: (usize, usize), mine_count: usize) -> Self {
        Self {
            running: false,
            game: Game::new(size, mine_count),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        terminal.hide_cursor()?;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let (game_width, game_height) = self.game.size;

        let board_width = 2 + (3 * game_width) as u16;
        let board_height = 2 + game_height as u16;

        let (time, status) = self.game.status();
        let secs = time.as_secs() % 60;
        let mins = (time.as_secs() - secs) / 60;

        let hud = [Line::default().spans([
            format!("{mins}:{secs:02}").white(),
            " ".gray(),
            format!("{}", self.game.unflagged_bombs()).on_red(),
            " ".gray(),
            match status {
                game::GameStatus::Initial => ":)",
                game::GameStatus::Won => ":D",
                game::GameStatus::Lost => ":(",
                game::GameStatus::Ongoing => ":o",
            }
            .white(),
        ])];

        let board_area = Rect::new(
            (frame.area().width - board_width) / 2,
            (frame.area().height - board_height) / 2 - hud.len() as u16,
            board_width,
            board_height,
        );
        {
            let board = Block::bordered().border_type(ratatui::widgets::BorderType::Rounded);
            let board_inner_area = board.inner(board_area);

            let hori = Layout::default()
                .constraints((0..game_width).map(|_| Constraint::Length(3)))
                .direction(Direction::Horizontal)
                .split(board_inner_area);

            for (x, hori) in hori.iter().enumerate() {
                let vert = Layout::default()
                    .constraints((0..game_height).map(|_| Constraint::Length(1)))
                    .direction(Direction::Vertical)
                    .split(*hori);

                for (y, hori) in vert.iter().enumerate() {
                    frame.render_widget(
                        Paragraph::new(
                            self.game
                                .tile_at(x, y)
                                .render_tile(x == self.game.cursor.0 && y == self.game.cursor.1),
                        )
                        .block(Block::new().on_black())
                        .centered(),
                        *hori,
                    );
                }
            }
            frame.render_widget(board, board_area);
        }
        let text_y = board_area.y + board_area.height;
        for (offset, hud) in hud.iter().enumerate() {
            let area = Rect::new(
                (frame.area().width - hud.width() as u16) / 2,
                text_y + offset as u16,
                hud.width() as u16,
                1,
            );
            frame.render_widget(hud, area);
        }
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        if !event::poll(std::time::Duration::from_millis(50))? {
            return Ok(());
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            _ => {}
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Up | KeyCode::Char('w')) => self.game.move_cursor(&CursorDirection::Up),
            (_, KeyCode::Left | KeyCode::Char('a')) => {
                self.game.move_cursor(&CursorDirection::Left);
            }
            (_, KeyCode::Down | KeyCode::Char('s')) => {
                self.game.move_cursor(&CursorDirection::Down);
            }
            (_, KeyCode::Right | KeyCode::Char('d')) => {
                self.game.move_cursor(&CursorDirection::Right);
            }
            (_, KeyCode::Char(' ')) => self.game.flag(),
            (_, KeyCode::Enter) => self.game.reveal(),
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
