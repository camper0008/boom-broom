use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rand::Rng;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Stylize},
    text::{Text, ToText},
    widgets::{Block, Paragraph},
};

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

enum TileContent {
    Mine,
    Field(u8),
}

enum TileMode {
    Hidden,
    Flagged,
    Revealed,
}

struct Tile {
    mode: TileMode,
    content: TileContent,
}

impl ToText for Tile {
    fn to_text(&self) -> Text<'_> {
        let ch = match (&self.mode, &self.content) {
            (TileMode::Hidden, _) => "-".to_string(),
            (TileMode::Flagged, _) => "î".to_string(),
            (TileMode::Revealed, TileContent::Mine) => "*".to_string(),
            (TileMode::Revealed, TileContent::Field(0)) => " ".to_string(),
            (TileMode::Revealed, TileContent::Field(n)) => n.to_string(),
        };
        let text = Text::raw(ch);
        text
    }
}

pub struct App {
    running: bool,
    tiles: Option<Tiles>,
}

struct Tiles {
    inner: Vec<Vec<Tile>>,
}

impl Tiles {
    fn valid_neighbours(&self, x: usize, y: usize) -> Vec<(isize, isize)> {
        (-1..=1)
            .flat_map(|x| (-1..=1).map(move |y| (x, y)))
            .filter(|&(x_offset, y_offset)| {
                let invalid = (x_offset == 0 && y_offset == 0)
                    || (x_offset < 0 && x == 0)
                    || (y_offset < 0 && y == 0)
                    || (x_offset > 0 && x == self.inner.len() - 1)
                    || (y_offset > 0 && y == self.inner[x].len() - 1);
                !invalid
            })
            .collect::<Vec<_>>()
    }
    fn blank((width, height): (usize, usize)) -> Vec<Vec<Tile>> {
        Vec::from_iter((0..width).map(|_| {
            Vec::from_iter((0..height).map(|_| Tile {
                mode: TileMode::Revealed,
                content: TileContent::Field(0),
            }))
        }))
    }
    fn populate_mines(&mut self, mine_count: usize, ignore: (usize, usize)) {
        let mut rng = rand::rng();
        for _ in 0..mine_count {
            loop {
                let x = rng.random_range(0..self.inner.len());
                let y = rng.random_range(0..self.inner[0].len());
                if (x, y) == ignore {
                    continue;
                }
                if matches!(self.inner[x][y].content, TileContent::Mine) {
                    continue;
                }
                self.inner[x][y].content = TileContent::Mine;
                break;
            }
        }
    }
    fn new((width, height): (usize, usize), mine_count: usize) {
        assert!(
            width * height >= mine_count,
            "should at most place `width*height` # of mines"
        );
        for x in 0..width {
            for y in 0..height {
                if !matches!(tiles[x][y].content, TileContent::Field(_)) {
                    continue;
                }
                let mut count = 0;
                {
                    let neighbours = self.neighbours(x, y);
                    let positions = (0..width)
                        .map(|x| (0..height).map(|y| (x, y)).collect::<Vec<_>>())
                        .collect::<Vec<_>>();

                    let (x, y) = (x as isize, y as isize);
                    let (width, height) = (width as isize, height as isize);
                    for x_offset in -1..=1 {
                        for y_offset in -1..=1 {
                            if x_offset == 0 && y_offset == 0 {
                                continue;
                            };
                            let (x, y) = (x + x_offset, y + y_offset);
                            if x < 0 || y < 0 || x >= width || y >= height {
                                continue;
                            };
                            let (x, y) = (x as usize, y as usize);
                            if matches!(&tiles[x][y].content, TileContent::Mine) {
                                count += 1;
                            }
                        }
                    }
                }
                tiles[x][y].content = TileContent::Field(count);
            }
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            running: false,
            tiles,
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
        let hori = Layout::default()
            .constraints(self.tiles.iter().map(|_| Constraint::Length(3)))
            .direction(Direction::Horizontal)
            .split(frame.area());

        for (x, hori) in hori.iter().enumerate() {
            let vert = Layout::default()
                .constraints(self.tiles[x].iter().map(|_| Constraint::Length(1)))
                .direction(Direction::Vertical)
                .split(*hori);

            for (y, hori) in vert.iter().enumerate() {
                frame.render_widget(
                    Paragraph::new(self.tiles[x][y].to_text()).block(Block::new().bg(
                        if x % 2 == y % 2 {
                            Color::DarkGray
                        } else {
                            Color::Gray
                        },
                    )),
                    *hori,
                );
            }
        }
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

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),

            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
