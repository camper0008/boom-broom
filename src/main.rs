use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    widgets::{Block, Paragraph},
};

use crate::tiles::{Tile, TileContent, TileMode, TileState, Tiles, TilesOptions};
mod tiles;

fn main() -> Result<()> {
    let [width, height, mines]: [usize; 3] = std::env::args()
        .skip(1)
        .map(|v| v.parse::<usize>().context("parameters should be usize"))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| eyre!("should only give two size parameters and one mine count parameter"))?;

    color_eyre::install()?;
    let mut terminal = ratatui::init();
    terminal.hide_cursor()?;
    let result = App::new((width, height), mines).run(terminal);
    ratatui::restore();
    result
}

trait SomeRenderThing {
    fn size(&self) -> (usize, usize);
    fn tile_at(&self, x: usize, y: usize) -> &Tile;
}

impl SomeRenderThing for TileState {
    fn size(&self) -> (usize, usize) {
        match &self {
            Self::Tiles(tiles) => (tiles.len(), tiles[0].len()),
            Self::Blank { width, height } => (*width, *height),
        }
    }

    fn tile_at(&self, x: usize, y: usize) -> &Tile {
        match &self {
            TileState::Blank { .. } => &Tile {
                mode: TileMode::Hidden,
                content: TileContent::Mine,
            },
            TileState::Tiles(tiles) => &tiles[x][y],
        }
    }
}

struct App {
    running: bool,
    tiles: TileState,
    mine_count: usize,
    cursor: (usize, usize),
}

enum CursorDirection {
    Up,
    Left,
    Right,
    Down,
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
            tiles: TileState::Blank {
                width: size.0,
                height: size.1,
            },
            mine_count,
            cursor: (0, 0),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let (width, height) = self.tiles.size();

        let border = Block::bordered()
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title("boombroom");

        let inner = border.inner(frame.area());

        let hori = Layout::default()
            .constraints((0..width).map(|_| Constraint::Length(3)))
            .direction(Direction::Horizontal)
            .split(inner);

        for (x, hori) in hori.iter().enumerate() {
            let vert = Layout::default()
                .constraints((0..height).map(|_| Constraint::Length(1)))
                .direction(Direction::Vertical)
                .split(*hori);

            for (y, hori) in vert.iter().enumerate() {
                frame.render_widget(
                    Paragraph::new(
                        self.tiles
                            .tile_at(x, y)
                            .render_tile(x == self.cursor.0 && y == self.cursor.1),
                    )
                    .block(Block::new().on_black())
                    .centered(),
                    *hori,
                );
            }
        }

        frame.render_widget(border, frame.area());
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    fn try_flag(&mut self) {
        match &mut self.tiles {
            tiles @ TileState::Blank { .. } => {
                *tiles = TileState::Tiles(Tiles::new(TilesOptions {
                    size: tiles.size(),
                    starting_position: self.cursor,
                    mine_count: self.mine_count,
                }));
            }
            TileState::Tiles(tiles) => {
                let tile = &mut tiles[self.cursor.0][self.cursor.1];
                tile.mode = match tile.mode {
                    TileMode::Hidden => TileMode::Flagged,
                    TileMode::Flagged => TileMode::Hidden,
                    TileMode::Revealed => TileMode::Revealed,
                }
            }
        }
    }
    fn try_reveal(&mut self) {
        match &mut self.tiles {
            tiles @ TileState::Blank { .. } => {
                *tiles = TileState::Tiles(Tiles::new(TilesOptions {
                    size: tiles.size(),
                    starting_position: self.cursor,
                    mine_count: self.mine_count,
                }));
            }
            TileState::Tiles(tiles) => {
                tiles.reveal(self.cursor.0, self.cursor.1);
            }
        }
    }

    fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Up => self.cursor.1 = self.cursor.1.saturating_sub(1),
            CursorDirection::Down => self.cursor.1 = self.cursor.1.saturating_add(1),
            CursorDirection::Left => self.cursor.0 = self.cursor.0.saturating_sub(1),
            CursorDirection::Right => self.cursor.0 = self.cursor.0.saturating_add(1),
        }
        let size = self.tiles.size();
        self.cursor.0 = self.cursor.0.clamp(0, size.0 - 1);
        self.cursor.1 = self.cursor.1.clamp(0, size.1 - 1);
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            (_, KeyCode::Up | KeyCode::Char('w')) => self.move_cursor(CursorDirection::Up),
            (_, KeyCode::Left | KeyCode::Char('a')) => self.move_cursor(CursorDirection::Left),
            (_, KeyCode::Down | KeyCode::Char('s')) => self.move_cursor(CursorDirection::Down),
            (_, KeyCode::Right | KeyCode::Char('d')) => self.move_cursor(CursorDirection::Right),
            (_, KeyCode::Char(' ')) => self.try_flag(),
            (_, KeyCode::Enter) => self.try_reveal(),
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
