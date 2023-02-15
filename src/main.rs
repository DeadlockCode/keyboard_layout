use std::fmt::Debug;
use std::fs;

use hashbrown::HashMap;
use rand::seq::SliceRandom;
use rand::{self, Rng};

use threadpool::ThreadPool;
use std::sync::{mpsc, Arc};

#[derive(Clone)]
struct Keyboard {
    keys: [u8; 26],
    distance: [[Option<u32>; 26]; 26],
    fitness: f32,
    finger_to_pos: [u8; 8],
    pos_to_finger: [u8; 26],
    total_distance: u32,
}

impl Keyboard {
    fn new() -> Self {
        Self {
            keys: [0; 26],
            distance: [[None; 26]; 26],
            fitness: 0.0,
            finger_to_pos: [0; 8],
            pos_to_finger: [0; 26],
            total_distance: u32::MAX,
        }
    }

    fn new_random() -> Self {
        let mut keyboard = Self::new();

        for i in 0..26 {
            keyboard.keys[i] = i as u8;
        }
        let mut rng = rand::thread_rng();
        keyboard.keys.shuffle(&mut rng);
        keyboard.finger_to_pos = [8, 9, 10, 11, 14, 15, 16, 17];

        keyboard.init_distance_tree();

        keyboard.pos_to_finger = [
               1, 2, 3, 3, 4, 4, 5, 6, 
            0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 
            0, 1, 2, 3,       4, 5, 6, 7, 
        ];

        keyboard
    }

    fn from_layout(keys: [char; 26]) -> Self {
        let mut keyboard = Self::new();

        for i in 0..26 {
            keyboard.keys[(keys[i] as u8 - b'a') as usize] = i as u8;
        }

        keyboard.finger_to_pos = [8, 9, 10, 11, 14, 15, 16, 17];
        keyboard.init_distance_tree();

        keyboard.pos_to_finger = [
               1, 2, 3, 3, 4, 4, 5, 6, 
            0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 
            0, 1, 2, 3,       4, 5, 6, 7, 
        ];

        keyboard
    }

    fn init_distance_tree(&mut self) {
        let mut dist = HashMap::new();

        // LP
        dist.insert((8, 18), 1000);

        // LR
        dist.insert((0, 9), 1000);
        dist.insert((9, 19), 1000);
        dist.insert((0, 19), 2000);

        // LM
        dist.insert((1, 10), 1000);
        dist.insert((10, 20), 1000);
        dist.insert((1, 20), 2000);

        // LI
        dist.insert((2, 11), 1000);
        dist.insert((11, 21), 1000);
        dist.insert((2, 21), 2000);
        
        dist.insert((3, 12), 1000);

        dist.insert((2, 3), 1000);
        dist.insert((11, 12), 1000);

        dist.insert((2, 12), 1414);
        dist.insert((3, 11), 1414);

        dist.insert((12, 21), 1414);

        dist.insert((3, 21), 2236);

        // RP
        dist.insert((17, 25), 1000);

        // RR
        dist.insert((7, 16), 1000);
        dist.insert((16, 24), 1000);
        dist.insert((7, 24), 2000);

        // RM
        dist.insert((6, 15), 1000);
        dist.insert((15, 23), 1000);
        dist.insert((6, 23), 2000);

        // RI
        dist.insert((5, 14), 1000);
        dist.insert((14, 22), 1000);
        dist.insert((5, 22), 2000);
        
        dist.insert((4, 13), 1000);

        dist.insert((4, 5), 1000);
        dist.insert((13, 14), 1000);

        dist.insert((4, 14), 1414);
        dist.insert((5, 13), 1414);

        dist.insert((13, 22), 1414);

        dist.insert((4, 22), 2236);

        for (key, &value) in dist.iter() {
            self.distance[key.0][key.1] = Some(value);
            self.distance[key.1][key.0] = Some(value);
        }
        for i in 0..26 {
            self.distance[i][i] = Some(0);
        }
    }

    fn fitness(&mut self, string: &str) {
        self.fitness = 0.0;

        let mut total_distance = 0;
        let mut repeat_fingers = 0;
        let mut repeat_hands = 0;
        let mut distribution = [0; 8];

        let mut total_keys = 0;

        let mut j = 0;

        string.chars().filter_map(|target| {
            if target == '\n' {
                return Some(&u8::MAX);
            }
            self.keys.get((target as u8 - b'a') as usize)
        }).for_each(|&target| {
            if target == u8::MAX {
                self.finger_to_pos = [8, 9, 10, 11, 14, 15, 16, 17];
                return;
            }
            total_keys += 1;

            let i = self.pos_to_finger[target as usize] as usize;
            distribution[i] += 1;

            let source = self.finger_to_pos[i];

            total_distance += self.distance[source as usize][target as usize].unwrap();

            self.finger_to_pos[i] = target;

            if target != source {
                if i == j {
                    repeat_fingers += 1;
                    repeat_hands += 1;
                }
                else if (i < 4 && j < 4) || (i >= 4 && j >= 4) {
                    repeat_hands += 1;
                }
            }

            j = i;
        });
        self.total_distance = total_distance;

        let distance_weight = 50.0;
        let finger_weight = 1.0;
        let hand_weight = 6.0;
        let deviation_weight = 15.0;

        let distance = total_keys as f32 / (total_distance as f32 / 1000.0) * distance_weight;
        let repeat_fingers = total_keys as f32 / repeat_fingers as f32 * finger_weight;
        let repeat_hands = total_keys as f32 / repeat_hands as f32 * hand_weight;
        let distribution = distribution.map(|value| {
            value as f32 / total_keys as f32
        });
        let mut deviation = 1.0 * deviation_weight;
        let desired_distribution = [0.0, 0.14, 0.14, 0.22, 0.22, 0.14, 0.14, 0.0];
        for i in 0..8 {
            deviation -= (desired_distribution[i] - distribution[i]).abs() * deviation_weight;
        }
        //println!("{}, {}, {}, {}", distance, repeat_fingers, repeat_hands, deviation);
        self.fitness = distance + repeat_fingers + repeat_hands + deviation;
    }

    fn mutate(&mut self) {
        let mut chance = 1.0;
        loop {
            let mutate = rand::thread_rng().gen::<f32>();
            if mutate < chance {
                let char1 = rand::thread_rng().gen_range(0..26);
                let char2 = rand::thread_rng().gen_range(0..26);
        
                self.keys.swap(char1, char2);
            }
            else {
                break;
            }

            chance *= 0.75;
        }

        self.finger_to_pos = [8, 9, 10, 11, 14, 15, 16, 17];
    }
}

impl Debug for Keyboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut invert = [0u8; 26];

        for i in 0..26 {
            invert[self.keys[i] as usize] = i as u8;
        }


        let mut data = String::new();
        for i in 0..26 {
            match i {
                0 => data += " ",
                4 => data += "  ",
                8 => data += "\n",
                13 => data += "  ",
                18 => data += &(" - Fitness: ".to_owned() + &(self.fitness).to_string() + "\n"),
                22 => data += "    ",
                _ => {},
            }
            data.push((invert[i] + b'A') as char);
        }
        write!(f, "{}\n", &data)
    }
}

fn read_dataset(path: &str) -> String {
    fs::read_to_string(path).expect(format!("Could not open: \"{}\"", path).as_str())
}


fn main() {
    let string = Arc::new(read_dataset("data\\dataset.txt")[0..100000].to_owned());

    let pool = ThreadPool::new(8);
    let (tx, rx) = mpsc::channel::<Keyboard>();

    let mut population = Vec::new();

    // Generate random population
    for _ in 0..1000 {
        let tx = tx.clone();
        let string = Arc::clone(&string);
        pool.execute(move || {
            let mut kb = Keyboard::new_random();
            kb.fitness(&string);
            tx.send(kb).expect("Awooga");
        });
    }
    rx.iter().take(1000).for_each(|kb: Keyboard| {
        population.push(kb);
    });

    // Generation loop
    loop {
        // Sorts the population based on fitness
        population.sort_by(|a, b| {
            a.fitness.partial_cmp(&b.fitness).unwrap().reverse()
        });

        // Prints the best keyboard
        println!("{:?}", population[0]);

        // Selects the 100 best from the population
        population = population[0..100].to_vec();
        for i in 0..100 {
            for _ in 0..9 {
                // Clone keyboard
                let mut kb = population[i].clone();

                // Start Multithreading
                let tx = tx.clone();
                let string = Arc::clone(&string);
                pool.execute(move || {
                    kb.mutate();
                    kb.fitness(&string);
                    // Send the keyboard from worker thread to main thread
                    tx.send(kb).expect("");
                });
            }
        }

        // Collect workerthread results
        rx.iter().take(900).for_each(|kb: Keyboard| {
            population.push(kb);
        });
    }
}