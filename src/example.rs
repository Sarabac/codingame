use std::io;

macro_rules! parse_input {
    ($x:expr, $t:ident) => {
        $x.trim().parse::<$t>().unwrap()
    };
}

/**
 * Grow and multiply your organisms to end up larger than your opponent.
 **/
fn main() {
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    let inputs = input_line.split(" ").collect::<Vec<_>>();
    let width = parse_input!(inputs[0], i32); // columns in the game grid
    let height = parse_input!(inputs[1], i32); // rows in the game grid

    // game loop
    loop {
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let entity_count = parse_input!(input_line, i32);
        for i in 0..entity_count as usize {
            let mut input_line = String::new();
            io::stdin().read_line(&mut input_line).unwrap();
            let inputs = input_line.split(" ").collect::<Vec<_>>();
            let x = parse_input!(inputs[0], i32);
            let y = parse_input!(inputs[1], i32); // grid coordinate
            let _type = inputs[2].trim().to_string(); // WALL, ROOT, BASIC, TENTACLE, HARVESTER, SPORER, A, B, C, D
            let owner = parse_input!(inputs[3], i32); // 1 if your organ, 0 if enemy organ, -1 if neither
            let organ_id = parse_input!(inputs[4], i32); // id of this entity if it's an organ, 0 otherwise
            let organ_dir = inputs[5].trim().to_string(); // N,E,S,W or X if not an organ
            let organ_parent_id = parse_input!(inputs[6], i32);
            let organ_root_id = parse_input!(inputs[7], i32);
        }
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let inputs = input_line.split(" ").collect::<Vec<_>>();
        let my_a = parse_input!(inputs[0], i32);
        let my_b = parse_input!(inputs[1], i32);
        let my_c = parse_input!(inputs[2], i32);
        let my_d = parse_input!(inputs[3], i32); // your protein stock
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let inputs = input_line.split(" ").collect::<Vec<_>>();
        let opp_a = parse_input!(inputs[0], i32);
        let opp_b = parse_input!(inputs[1], i32);
        let opp_c = parse_input!(inputs[2], i32);
        let opp_d = parse_input!(inputs[3], i32); // opponent's protein stock
        let mut input_line = String::new();
        io::stdin().read_line(&mut input_line).unwrap();
        let required_actions_count = parse_input!(input_line, i32); // your number of organisms, output an action for each one in any order
        let mut idx = 1;
        for i in 0..required_actions_count as usize {
            // Write an action using println!("message...");
            // To debug: eprintln!("Debug message...");

            println!("GROW {idx} 16 2 BASIC");
            idx += 1;
        }
    }
}
