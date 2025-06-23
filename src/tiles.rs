use std::ops::{Deref, DerefMut};

use rand::Rng;

pub enum TileContent {
    Mine,
    Field(u8),
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

pub enum TileState {
    Blank { width: usize, height: usize },
    Tiles(Tiles),
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

impl Tiles {
    pub fn is_dead(&self) -> bool {
        self.iter().flatten().any(|v| {
            matches!(
                (&v.mode, &v.content),
                (TileMode::Revealed, TileContent::Mine)
            )
        })
    }
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
        Tiles(Vec::from_iter((0..width).map(|_| {
            Vec::from_iter((0..height).map(|_| Tile {
                mode: TileMode::Hidden,
                content: TileContent::Field(0),
            }))
        })))
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
    pub fn reveal(&mut self, x: usize, y: usize) {
        let tile = &mut self[x][y];
        match tile.mode {
            TileMode::Hidden => tile.mode = TileMode::Revealed,
            TileMode::Flagged => return,
            TileMode::Revealed => return,
        }
        if let TileContent::Field(0) = tile.content {
            let nbs = self.neighbours(x, y);
            for nb_pos in nbs {
                self.reveal(nb_pos.0, nb_pos.1);
            }
        }
    }
    pub fn new(options: TilesOptions) -> Self {
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
