use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize)]
pub struct Config {
    server: Server,
    problems: Vec<Problem>,
    languages: Vec<Language>,
}

#[derive(Deserialize, Serialize)]
struct Server {
    bind_address: Option<String>,
    bind_port: Option<u32>,
}

#[derive(Deserialize, Serialize)]
struct Problem {
    id: u32,
    name: String,
    r#type: String,
    cases: Vec<Case>,
}

#[derive(Deserialize, Serialize)]
struct Case {
    score: f32,
    input_file: String,
    answer_file: String,
    time_limit: u32,
    memory_limit: u32,
}

#[derive(Deserialize, Serialize)]
struct Language {
    name: String,
    file_name: String,
    command: Vec<String>,
}
