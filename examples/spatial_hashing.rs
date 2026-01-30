#![allow(dead_code)]
#![allow(unused)]
use std::collections::HashMap;
// TODO add errors

fn main() {
    // make sensible bounds for demo
    let bounds = Bounds::new_from_indices(0, 0, 100, 100);
    let cell_size = Dimensions {
        width: 10.0,
        height: 10.0,
    };
    let mut grid = SpatialHash::new(cell_size, bounds);

    let pos = Vec2::new(15.0, 15.0);
    let dimensions = Dimensions {
        width: 4.0,
        height: 4.0,
    };

    let mut ent = grid.create(pos, dimensions);
    grid.update(&mut ent);

    println!("\n\n");
    println!("grid.cells = {:#?}", grid.cells);
}

#[derive(Default, Clone, Debug)]
pub struct Vec2 {
    x: f32,
    y: f32,
}
impl Vec2 {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Default, Debug)]
pub struct Bounds {
    start: Cell,
    end: Cell,
}
impl Bounds {
    fn new(start: Cell, end: Cell) -> Self {
        Self { start, end }
    }
    fn new_from_indices(start_x: i32, start_y: i32, end_x: i32, end_y: i32) -> Self {
        let start = Cell::new(start_x, start_y);
        let end = Cell::new(end_x, end_y);
        Self { start, end }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug, Default)]
struct Cell {
    x: i32,
    y: i32,
}
impl Cell {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

// this should be a private type, since the id only gets propagated via the hashgrid
pub struct Entity {
    pos: Vec2,
    dimensions: Dimensions,
    cells: Bounds,
    id: u32,
}
impl Entity {
    fn new(pos: Vec2, dimensions: Dimensions, cells: Bounds, id: u32) -> Self {
        Self {
            pos,
            dimensions,
            cells,
            id,
        }
    }
}

#[derive(Default, Debug)]
pub struct Dimensions {
    width: f32,
    height: f32,
}

pub struct SpatialHash {
    cells: HashMap<Cell, Vec<u32>>, // Cellindex + uuids
    bounds: Bounds,
    dimensions: Dimensions,
    id: u32,
}

impl SpatialHash {
    pub fn new(cell_size: Dimensions, bounds: Bounds) -> Self {
        let cells = HashMap::default();
        let dimensions_x = (bounds.end.x - bounds.start.x) as f32 / cell_size.width;
        let dimensions_y = (bounds.end.y - bounds.start.y) as f32 / cell_size.height;
        let dimensions = Dimensions {
            width: dimensions_x,
            height: dimensions_y,
        };
        Self {
            cells,
            bounds,
            dimensions,
            id: 0,
        }
    }

    // this should give the client sadly...
    pub fn create(self: &mut Self, pos: Vec2, dimensions: Dimensions) -> Entity {
        dbg! {&dimensions};
        let bounds = self.compute_cell_bounds(&pos, &dimensions);
        dbg! {&bounds};

        let entity = Entity::new(pos, dimensions, bounds, self.id);
        self.id += 1;
        self.insert(&entity).unwrap();
        return entity;
    }

    fn insert(self: &mut Self, entity: &Entity) -> anyhow::Result<(), anyhow::Error> {
        let Bounds { start, end } = &entity.cells;
        if start.x < self.bounds.start.x
            || start.y < self.bounds.start.y
            || end.x > self.bounds.end.x
            || end.y > self.bounds.end.y
        {
            return Err(anyhow::format_err!("Placed Entity out of Bounds!"));
        }

        for x in start.x..=end.x {
            for y in start.y..=end.y {
                let cell = Cell { x, y };
                self.cells.entry(cell).or_default().push(entity.id);
            }
        }
        Ok(())
    }

    fn compute_cell_bounds(&self, pos: &Vec2, dimensions: &Dimensions) -> Bounds {
        let pos_start = Vec2::new(
            pos.x - dimensions.width / 2.0,
            pos.y - dimensions.height / 2.0,
        );
        if self.bounds.end.x == self.bounds.start.x {
            // TODO throw error
            panic!("X Bounds are 0");
        }
        if self.bounds.end.y == self.bounds.start.y {
            // TODO throw error
            panic!("Y Bounds are 0");
        }
        // TODO ensure that this is between 0 and 1
        let relative_pos_x = pos_start.x
            - self.bounds.start.x as f32 / self.bounds.end.x as f32
            - self.bounds.start.x as f32;
        let relative_pos_y =
            pos_start.y - self.bounds.start.y / self.bounds.end.y - self.bounds.start.y;
        let start_index_x = relative_pos_x * self.dimensions.width;
        let start_index_y = relative_pos_y * self.dimensions.height;

        let pos_end = Vec2::new(
            pos.x + dimensions.width / 2.0,
            pos.y + dimensions.height / 2.0,
        );
        // TODO ensure that this is between 0 and 1
        let relative_pos_x =
            pos_end.x - self.bounds.start.x / self.bounds.end.x - self.bounds.start.x;
        let relative_pos_y =
            pos_end.y - self.bounds.start.y / self.bounds.end.y - self.bounds.start.y;
        let end_index_x = relative_pos_x * self.dimensions.width;
        let end_index_y = relative_pos_y * self.dimensions.height;

        Bounds::new_from_indices(start_index_x, start_index_y, end_index_x, end_index_y)
    }

    pub fn remove(self: &mut Self, bounds: &Bounds, id: u32) {
        let Bounds { start, end } = bounds;
        // dbg! {bounds};
        for x in start.x..=end.x {
            for y in start.y..=end.y {
                let cell = Cell { x, y };
                if let Some(vec) = self.cells.get_mut(&cell) {
                    vec.retain(|&other_id| other_id != id);
                    if vec.is_empty() {
                        self.cells.remove(&cell);
                    }
                }
            }
        }
    }

    pub fn update(self: &mut Self, entity: &mut Entity) {
        self.remove(&entity.cells, entity.id);
        entity.cells = self.compute_cell_bounds(&entity.pos, &entity.dimensions);
        self.insert(&entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{self, AssertUnwindSafe};

    #[test]
    fn create_one_entity() {
        // this way some cells are empty and some are doubly occupied
        let bounds = Bounds::new_from_indices(0, 0, 100, 100);
        let cell_size = Dimensions {
            width: 10,
            height: 10,
        };
        let mut grid = SpatialHash::new(cell_size, bounds);

        let pos = Vec2::new(42, 42);
        let dimensions = Dimensions {
            width: 1,
            height: 1,
        };
        let ent = grid.create(pos, dimensions);

        let cells_occupied: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(cells_occupied, 1, "expected 1 cell occopied after creation");
    }
    #[test]
    fn create_big_entity() {
        // this way some cells are empty and some are doubly occupied
        let bounds = Bounds::new_from_indices(0, 0, 100, 100);
        let cell_size = Dimensions {
            width: 10,
            height: 10,
        };
        let mut grid = SpatialHash::new(cell_size, bounds);

        let pos = Vec2::new(42, 42);
        let dimensions = Dimensions {
            width: 4,
            height: 4,
        };
        let ent = grid.create(pos, dimensions);

        let cells_occupied: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(cells_occupied, 4, "expected 4 cell occopied after creation");
    }
    #[test]
    fn create_big_entity_odd() {
        // this way some cells are empty and some are doubly occupied
        let bounds = Bounds::new_from_indices(0, 0, 100, 100);
        let cell_size = Dimensions {
            width: 10,
            height: 10,
        };
        let mut grid = SpatialHash::new(cell_size, bounds);

        let pos = Vec2::new(42, 42);
        let dimensions = Dimensions {
            width: 2,
            height: 3,
        };
        let ent = grid.create(pos, dimensions);

        let cells_occupied: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(cells_occupied, 6, "expected 6 cell occopied after creation");
    }
    #[test]
    fn create_entity_out_of_bounds() {
        // this way some cells are empty and some are doubly occupied
        let bounds = Bounds::new_from_indices(0, 0, 100, 100);
        let cell_size = Dimensions {
            width: 10,
            height: 10,
        };
        let mut grid = SpatialHash::new(cell_size, bounds);

        let pos = Vec2::new(-1, 102);
        let dimensions = Dimensions {
            width: 1,
            height: 1,
        };
        // Act & Assert
        let res = panic::catch_unwind(AssertUnwindSafe(|| {
            grid.create(pos, dimensions);
        }));
        assert!(res.is_err(), "expected an out of bounds panic");
    }
    #[test]
    fn test_spatialhash_with_1000_entities() {
        // this way some cells are empty and some are doubly occupied
        let bounds = Bounds::new_from_indices(0, 0, 100, 100);
        let cell_size = Dimensions {
            width: 10,
            height: 10,
        };
        let mut grid = SpatialHash::new(cell_size, bounds);

        let mut entities: Vec<Entity> = Vec::with_capacity(1000);
        for i in 0..1000u32 {
            // deterministic positioning using simple arithmetic so we don't need rand crate
            // using 98, so that bigger values don't get placed in a wall.
            let x = (((i as i32) * 17) % 77);
            let y = (((i as i32) * 3) % 77);
            let pos = Vec2::new(x, y);
            let dimensions = Dimensions {
                width: 4,
                height: 4,
            };
            let ent = grid.create(pos, dimensions);
            entities.push(ent);
        }

        // After insertion, total ids stored across all cells should equal the number of created entities
        let total_ids: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(total_ids, 1000, "expected 1000 ids after creation");

        // consume (take ownership of) the first 200 entities and call update on them.
        // The API in this module's update simply removes and reinserts using the provided entity.
        // We call update to exercise that path; we remove entities from our local vector as we pass ownership.
        for _ in 0..200 {
            if entities.is_empty() {
                assert!(false, "Entities was empty");
                break;
            }
            // remove first element and pass it to update (consumes the Entity)
            let mut ent = entities.remove(0);
            // mutate position slightly (not necessary for this API to run, but exercises changing data)
            ent.pos.x += 5;
            ent.pos.y += 5;
            // call update which will remove and re-insert the id (no panic expected)
            grid.update(&mut ent);
        }

        let total_after_update: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(total_after_update, 1000, "expected 1000 ids after updates");

        // remove roughly a third of the remaining entities by calling remove(&Entity) for every 3rd entry
        // entities vector currently holds 800 entries (we removed 200 by moving them into update above).
        let mut removed_count = 0usize;
        let mut idx = 0usize;
        while idx < entities.len() {
            if idx % 3 == 0 {
                let ent_ref = &entities[idx];
                grid.remove(&ent_ref.cells, ent_ref.id);
                removed_count += 1;
            }
            idx += 1;
        }

        // total ids now should equal initial_count - removed_count
        let total_after_removals: usize = grid.cells.values().map(|v| v.len()).sum();
        assert_eq!(
            total_after_removals,
            1000 - removed_count,
            "expected ids after removals to match"
        );

        // sanity checks on internal map: no cell should contain duplicate ids for this usage pattern
        for (cell, vec) in grid.cells.iter() {
            // check for duplicates by converting to a set-like check
            let mut seen = std::collections::HashMap::<u32, usize>::new();
            for id in vec.iter() {
                let counter = seen.entry(*id).or_insert(0usize);
                *counter += 1;
            }
            for (id, count) in seen.into_iter() {
                assert!(
                    count == 1,
                    "duplicate id {} found in cell {:?} (count = {})",
                    id,
                    cell,
                    count
                );
            }
        }
    }
}
