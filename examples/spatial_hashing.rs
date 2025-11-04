#![allow(dead_code)]
#![allow(unused)]
use std::collections::HashMap;

fn main() {
    let bounds = Bounds::default();
    let dimensions = Position::default();
    let mut grid = SpatialHashGrid::new(bounds.clone(), dimensions);

    let entity = Entity::default();
    let mut client = grid.new_client(Position::default(), 10);


    client.pos = Position::default();
    grid.update_client(client);

    let position = Position::default();
    let nearby = grid.find_nearby(position, bounds);

    grid.remove_client(client);
    println!("hey tiger!");
}

#[derive(Clone)]
struct Bounds {
    beginning: Position,
    end: Position,
}

impl Bounds {
    fn default() -> Self{
        Self{beginning: Position::default(), end: Position::default()}
    }
}

#[derive(Clone, Copy, Hash)]
struct Position {
    x: i32,
    y: i32,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        let res = self.x == other.x && self.y == other.y;
        return res;

    }

}

impl Eq for Position {}


impl Position {
    fn new(x: i32, y: i32) -> Self {
        Self{x, y}
    }

    fn default() -> Self {
        Self{x: 0, y: 0}
    }
}

#[derive(Copy, Clone)]
struct Entity {
    pos: Position,
    radius: u32,
    id: u32,
}

impl Entity {
    fn new(pos: Position, radius: u32, id: u32) -> Self {
        Self{ pos, radius, id}
    }
    fn default() -> Self {
        Self{ pos: Position::default(), radius: 20, id: 1}
    }

}
fn get_nearby(entities: &[Entity], position: Position, radius: u32) -> Entity {
    for i in 0..entities.len() {
        let e = &entities[i];

        /*
        if distance(e.pos, position) < (radius + e.radius) {
        }
        */

    }

    return Entity{pos: Position::default(), radius: 1, id: 0};
}

struct SpatialHashGrid {
    bounds: Bounds, // this is [[minX, minY], [maxX, maxY]]
    dimensions: Position, // TODO this is col rows
    cells: HashMap<u32, Position>,
}

impl SpatialHashGrid {
    fn new(bounds: Bounds, dimensions: Position) -> Self {
        let cells = HashMap::new();
        Self{bounds, dimensions, cells}
    }

    fn new_client(self: &mut Self, position: Position, dimension: Position) -> Entity {
        let client = Entity::new(position, dimension, 1);
        self.cells.insert(client.id, position);

        self.insert(client);
        client
    }

    fn update_client(self: &Self, parameter: Entity){

    }

    fn find_nearby(self: &Self, pos: Position, bounds: Bounds){

    }

    fn remove_client(self: &Self, parameter: Entity){

    }

    fn insert(self: &mut Self, client: Entity) {
        let Position{x,y} = client.pos;
        let Position{x:w, y:h} = self.dimensions;

        // x 25, w = 100 rows?
        // 25 - 50??
        let i1 = get_cell_index([x-w/2, y-h/2]);
        let i2 = get_cell_index([x+w/2, y+h/2]);

        client.indices = [i1, i2];

        for x in i1[0]..i2[0] {
            for y in i1[1]..i2[1] {
            }
        }


        

    }

    fn get_cell_index(pos: [u32]) -> u32{
        // x = sat of current pos (-beginning) / overall bounds (-beginning)
        
        // same for y should be a number between 0 and 1
        
        // xIndex = Math.floor(x * number of dimensions) so 0.25 * 100 = 25 
        
        // return xIndex, yIndex
    }
}
