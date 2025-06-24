use std::{
    ops::{Deref, DerefMut},
    time::{Duration, Instant},
};

use rand::Rng;

pub enum TileMistake {
    TrippedMine,
    FlaggedField(u8),
}

pub enum TileContent {
    Mine,
    Field(u8),
    Mistake(TileMistake),
}

pub enum TileMode {
    Hidden,
    Flagged,
    Revealed,
}

pub struct Tile {
    pub mode: TileMode,
    pub content: TileContent,
}

pub struct Tiles(Vec<Vec<Tile>>);

impl Deref for Tiles {
    type Target = Vec<Vec<Tile>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct TilesOptions {
    pub size: (usize, usize),
    pub starting_position: (usize, usize),
    pub mine_count: usize,
}

enum GameState {
    Blank,
    Ongoing { started: Instant, tiles: Tiles },
    Finished { took: Duration, tiles: Tiles },
}

pub struct Game {
    pub cursor: (usize, usize),
    pub size: (usize, usize),
    state: GameState,
    mine_count: usize,
}

pub enum CursorDirection {
    Up,
    Left,
    Right,
    Down,
}

pub enum GameStatus {
    Initial,
    Won,
    Lost,
    Ongoing,
}

impl Game {
    pub fn new(size: (usize, usize), mine_count: usize) -> Self {
        Self {
            cursor: (0, 0),
            size,
            state: GameState::Blank,
            mine_count,
        }
    }

    pub fn status(&self) -> (Duration, GameStatus) {
        match &self.state {
            GameState::Blank => (Duration::from_secs(0), GameStatus::Initial),
            GameState::Ongoing { started, .. } => {
                (Instant::now().duration_since(*started), GameStatus::Ongoing)
            }
            GameState::Finished { took, tiles } => {
                let lost = tiles
                    .iter()
                    .flatten()
                    .any(|tile| matches!(tile.content, TileContent::Mistake(_)));
                if lost {
                    (*took, GameStatus::Lost)
                } else {
                    (*took, GameStatus::Won)
                }
            }
        }
    }
    pub fn unflagged_bombs(&self) -> i32 {
        let (GameState::Ongoing { tiles, .. } | GameState::Finished { tiles, .. }) = &self.state
        else {
            return self.mine_count as i32;
        };
        let flags = tiles
            .iter()
            .flatten()
            .filter(|tile| matches!(tile.mode, TileMode::Flagged))
            .count() as i32;
        return self.mine_count as i32 - flags;
    }
    pub fn tile_at(&self, x: usize, y: usize) -> &Tile {
        let (GameState::Ongoing { tiles, .. } | GameState::Finished { tiles, .. }) = &self.state
        else {
            return &Tile {
                mode: TileMode::Hidden,
                content: TileContent::Field(0),
            };
        };
        &tiles[x][y]
    }
    fn finish_game(&mut self) {
        let GameState::Ongoing { tiles, started } = &mut self.state else {
            unreachable!();
        };
        let mut tiles = std::mem::replace(tiles, Tiles::new_blank((0, 0)));
        for tile in tiles.iter_mut().flatten() {
            match (&tile.mode, &tile.content) {
                (TileMode::Flagged, TileContent::Field(c)) => {
                    tile.content = TileContent::Mistake(TileMistake::FlaggedField(*c));
                }
                (TileMode::Flagged, TileContent::Mine)
                | (TileMode::Revealed | TileMode::Hidden, TileContent::Field(_)) => {}
                (TileMode::Revealed, TileContent::Mine) => {
                    tile.mode = TileMode::Revealed;
                    tile.content = TileContent::Mistake(TileMistake::TrippedMine);
                }
                (TileMode::Hidden, TileContent::Mine) => tile.mode = TileMode::Revealed,
                (_, TileContent::Mistake(_)) => unreachable!(),
            }
        }
        let took = Instant::now().duration_since(*started);
        self.state = GameState::Finished { took, tiles };
    }
    fn move_on(&mut self) {
        match self.state {
            GameState::Blank => {
                self.state = GameState::Ongoing {
                    started: Instant::now(),
                    tiles: Tiles::new(&TilesOptions {
                        size: self.size,
                        starting_position: self.cursor,
                        mine_count: self.mine_count,
                    }),
                };
            }
            GameState::Finished { .. } => self.state = GameState::Blank,
            GameState::Ongoing { .. } => unreachable!(),
        }
    }
    fn maybe_finish(&mut self) {
        let GameState::Ongoing { tiles, .. } = &self.state else {
            unreachable!();
        };
        let has_won = tiles.iter().flatten().all(|tile| {
            !matches!(tile.content, TileContent::Field(_))
                || matches!(tile.mode, TileMode::Revealed)
        });
        let has_lost = tiles.iter().flatten().any(|tile| {
            matches!(
                (&tile.mode, &tile.content),
                (TileMode::Revealed, TileContent::Mine)
            )
        });
        if has_won || has_lost {
            self.finish_game();
        }
    }

    pub fn flag(&mut self) {
        let GameState::Ongoing { tiles, .. } = &mut self.state else {
            self.move_on();
            return;
        };

        let tile = &mut tiles[self.cursor.0][self.cursor.1];
        tile.mode = match tile.mode {
            TileMode::Hidden => TileMode::Flagged,
            TileMode::Flagged => TileMode::Hidden,
            TileMode::Revealed => TileMode::Revealed,
        };
    }
    pub fn reveal(&mut self) {
        let GameState::Ongoing { tiles, .. } = &mut self.state else {
            self.move_on();
            return;
        };
        tiles.reveal(self.cursor.0, self.cursor.1);
        self.maybe_finish();
    }

    pub fn move_cursor(&mut self, direction: &CursorDirection) {
        match direction {
            CursorDirection::Up => self.cursor.1 = self.cursor.1.saturating_sub(1),
            CursorDirection::Down => self.cursor.1 = self.cursor.1.saturating_add(1),
            CursorDirection::Left => self.cursor.0 = self.cursor.0.saturating_sub(1),
            CursorDirection::Right => self.cursor.0 = self.cursor.0.saturating_add(1),
        }
        let size = self.size;
        self.cursor.0 = self.cursor.0.clamp(0, size.0 - 1);
        self.cursor.1 = self.cursor.1.clamp(0, size.1 - 1);
    }
}

impl Tiles {
    fn neighbours(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        (-1..=1)
            .flat_map(|x| (-1..=1).map(move |y| (x, y)))
            .filter(|&(x_offset, y_offset)| {
                let invalid = (x_offset == 0 && y_offset == 0)
                    || (x_offset < 0 && x == 0)
                    || (y_offset < 0 && y == 0)
                    || (x_offset > 0 && x == self.len() - 1)
                    || (y_offset > 0 && y == self[x].len() - 1);
                !invalid
            })
            .map(|(x_offset, y_offset)| (x as isize + x_offset, y as isize + y_offset))
            .map(|(x, y)| (x as usize, y as usize))
            .collect()
    }
    fn new_blank((width, height): (usize, usize)) -> Tiles {
        Tiles(
            ((0..width).map(|_| {
                ((0..height).map(|_| Tile {
                    mode: TileMode::Hidden,
                    content: TileContent::Field(0),
                }))
                .collect()
            }))
            .collect(),
        )
    }
    fn populate_mines(&mut self, mine_count: usize, ignore: (usize, usize)) {
        let mut rng = rand::rng();
        for _ in 0..mine_count {
            loop {
                let x = rng.random_range(0..self.len());
                let y = rng.random_range(0..self[0].len());
                if (x, y) == ignore {
                    continue;
                }
                if matches!(self[x][y].content, TileContent::Mine) {
                    continue;
                }
                self[x][y].content = TileContent::Mine;
                break;
            }
        }
    }
    fn reveal(&mut self, x: usize, y: usize) {
        let tile = &mut self[x][y];
        match tile.mode {
            TileMode::Hidden => {
                tile.mode = TileMode::Revealed;
                let TileContent::Field(0) = tile.content else {
                    return;
                };
            }
            TileMode::Flagged => return,
            TileMode::Revealed => {
                let TileContent::Field(mines) = tile.content else {
                    unreachable!()
                };
                let flags = self
                    .neighbours(x, y)
                    .iter()
                    .filter(|(x, y)| matches!(self[*x][*y].mode, TileMode::Flagged))
                    .count();
                if mines != flags as u8 {
                    return;
                }
            }
        }
        for nb_pos in self.neighbours(x, y) {
            let tile = &self[nb_pos.0][nb_pos.1];
            let TileMode::Hidden = tile.mode else {
                continue;
            };
            self.reveal(nb_pos.0, nb_pos.1);
        }
    }
    fn new(options: &TilesOptions) -> Self {
        let (width, height) = options.size;
        assert!(
            width * height > options.mine_count,
            "should at most place `width*height - 1` # of mines"
        );
        let mut tiles = Self::new_blank((width, height));
        tiles.populate_mines(options.mine_count, options.starting_position);
        for x in 0..width {
            for y in 0..height {
                if !matches!(tiles[x][y].content, TileContent::Field(_)) {
                    continue;
                }
                let mines = tiles
                    .neighbours(x, y)
                    .iter()
                    .filter(|(x, y)| matches!(&tiles[*x][*y].content, TileContent::Mine))
                    .count();
                tiles[x][y].content = TileContent::Field(mines as u8);
            }
        }
        tiles.reveal(options.starting_position.0, options.starting_position.1);
        tiles
    }
}
