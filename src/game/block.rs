#[derive(PartialEq)]
pub enum TileType {
    King = 0,     // base
    City = 1,     // spawner
    Fog = 2,      // it's color unit = null
    Obstacle = 3, // Either City or Mountain, which is unknown, it's color unit = null
    Plain = 4,    // blank , plain, Neutral, 有数值时，即是army
    Mountain = 5,
    Swamp = 6,
}

pub struct Block {
    x: i32,
    y: i32,
    tile_type: TileType,
    unit: i64,
    color: i16,
    team: i16,
    is_always_revealed: bool,
}

impl Block {
    pub fn init_king(mut self, color: i16) {
        self.color = color;
        self.tile_type = TileType::King;
        self.unit = 1;
    }

    pub fn dominated_by(mut self, color: i16, team: i16) {
        self.color = color;
        self.team = team;
    }

    pub fn entered_by(mut self, color: i16, team: i16, unit: i64) {
        if self.team == team {
            self.unit += unit;
            if self.tile_type != TileType::King {
                self.dominated_by(color, team);
            }
        } else {
            if self.unit >= unit {
                self.unit -= unit;
            } else if self.unit < unit {
                self.unit = unit - self.unit;
                self.dominated_by(color, team);
            }
        }
    }

    pub fn get_movable_unit(&self) -> i64 {
        return i64::max(self.unit - 1, 0);
    }
}
