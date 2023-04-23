pub enum Direction {
    Left,
    //Right,
}

pub fn rotate(str: &str, direction: Direction, count: usize) -> String {
    let mut str_vec: Vec<char> = str.chars().collect();
    match direction {
        Direction::Left => str_vec.rotate_left(count),
        //Direction::Right => str_vec.rotate_right(count),
    }
    str_vec.iter().collect()
}
