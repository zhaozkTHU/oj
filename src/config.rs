use std::fs::File;
use std::io::BufReader;

use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    server: Server,
    pub problems: Vec<Problem>,
    pub languages: Vec<Language>,
}

impl Config {
    pub fn parse_from_file(path: &String) -> Config {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let config: Config = serde_json::from_reader(reader).unwrap();
        return config;
    }
}

#[derive(Deserialize, Serialize, Clone)]
struct Server {
    bind_address: Option<String>,
    bind_port: Option<u32>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Problem {
    pub id: u32,
    name: String,
    pub r#type: String,
    pub misc: Misc,
    pub cases: Vec<Case>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Case {
    pub score: f32,
    pub input_file: String,
    pub answer_file: String,
    pub time_limit: u64,
    memory_limit: u32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Language {
    pub name: String,
    file_name: String,
    pub command: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Misc {
    pub packing: Option<Vec<Vec<u32>>>,
    pub special_judge: Option<Vec<String>>,
}
