use actix_web::web;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;
use std::{fs, io::Write, process::Command};
use wait_timeout::ChildExt;

use crate::config::Config;
use crate::jobs::{Case, Result};

pub fn judger(
    source_code: &String,
    index: usize,
    language: &String,
    config: &web::Data<Config>,
) -> (Vec<Case>, f32) {
    // Create temporart direction
    fs::create_dir("./TMPDIR").unwrap();
    let mut main_file: fs::File;
    match language.as_str() {
        "Rust" => main_file = fs::File::create("./TMPDIR/main.rs").unwrap(),
        "C" => main_file = fs::File::create("./TMPDIR/main.c").unwrap(),
        "C++" => main_file = fs::File::create("./TMPDIR/main.cpp").unwrap(),
        _ => unreachable!(),
    };
    main_file.write_all(source_code.as_bytes()).unwrap();

    // Compile
    let (compile_success, compile_time) = compile(config, language);

    let mut cases: Vec<Case> = Vec::new();
    if compile_success {
        cases.push(Case {
            id: 0,
            result: Result::CompilationSuccess,
            time: compile_time,
            memory: 0,
            info: "".to_string(),
        });
    } else {
        // Compilation Error
        for _ in config.problems[index].cases.iter() {
            cases.push(Case {
                id: 0,
                result: Result::CompilationError,
                time: compile_time,
                memory: 0,
                info: "".to_string(),
            })
        }
    }

    let scores: f32 = get_scores(config, &mut cases, index);

    fs::remove_dir_all("./TMPDIR").unwrap();
    return (cases, scores);
}

/// Compile according to language
/// return whether success and compile time
fn compile(config: &web::Data<Config>, language: &String) -> (bool, u128) {
    // Add arguments
    let mut args: Vec<String> = Vec::new();
    for j in config.languages.iter().enumerate() {
        if j.1.name == *language {
            for i in config.languages[j.0].command.iter().skip(1) {
                if i == "%OUTPUT%" {
                    args.push("./TMPDIR/main".to_string());
                    continue;
                }
                if i == "%INPUT%" {
                    match language.as_str() {
                        "Rust" => args.push("./TMPDIR/main.rs".to_string()),
                        "C" => args.push("./TMPDIR/main.c".to_string()),
                        "C++" => args.push("./TMPDIR/main.cpp".to_string()),
                        _ => unreachable!(),
                    };
                    continue;
                }
                args.push(i.clone());
            }
        }
    }

    // Compile
    let compile_start = Instant::now();
    let compile_status: std::process::ExitStatus;
    match language.as_str() {
        "Rust" => {
            compile_status = Command::new("rustc").args(args).status().unwrap();
        }
        "C" => {
            compile_status = Command::new("gcc").args(args).status().unwrap();
        }
        "C++" => {
            compile_status = Command::new("g++").args(args).status().unwrap();
        }
        _ => unreachable!(),
    }

    let compile_time = compile_start.elapsed().as_micros();
    (compile_status.success(), compile_time)
}

fn get_scores(config: &web::Data<Config>, cases: &mut Vec<Case>, problem_id: usize) -> f32 {
    let mut scores = 0.0;
    let mut id: u32 = 0;

    let mut packing: Vec<Vec<bool>> = Vec::new();
    if config.problems[problem_id].misc.packing.is_some() {
        for i in config.problems[problem_id]
            .misc
            .packing
            .clone()
            .unwrap()
            .iter()
        {
            packing.push(vec![false; i.len()]);
        }
    }
    let mut pack_score = vec![0 as f32; packing.len()];

    for i in config.problems[problem_id].cases.iter() {
        id += 1;

        // The case in which pack
        let mut packing_id = 0;
        // The case index in this pack
        let mut packing_index = 0;
        if !packing.is_empty() {
            let mut tmp = 0;
            for (res, v) in packing.iter().enumerate() {
                if id > tmp && id <= tmp + v.len() as u32 {
                    packing_id = res;
                    packing_index = id - tmp - 1;
                    break;
                } else {
                    tmp += v.len() as u32;
                }
            }
        }
        let mut skipped = false;
        for i in packing[packing_id].iter().take(packing_index as usize) {
            if !i {
                cases.push(Case {
                    id,
                    result: Result::Skipped,
                    time: 0,
                    memory: 0,
                    info: "".to_string(),
                });
                skipped = true;
                break;
            }
        }
        if skipped {
            continue;
        }

        // Compile error
        if fs::File::open("./TMPDIR/main").is_err() {
            cases.push(Case {
                id,
                result: Result::Waiting,
                time: 0,
                memory: 0,
                info: "".to_string(),
            });
            continue;
        }

        let in_file = fs::File::open(&i.input_file).unwrap();
        let out_file = fs::File::create("./TMPDIR/out").unwrap();

        // Run the executable file
        let run_start = Instant::now();
        let mut child = Command::new("./TMPDIR/main")
            .stdin(Stdio::from(in_file))
            .stdout(out_file)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let time_limit = Duration::from_micros(i.time_limit);
        let run_status = match child.wait_timeout(time_limit).unwrap() {
            Some(status) => status,
            None => {
                cases.push(Case {
                    id,
                    result: Result::TimeLimitExceeded,
                    time: 0,
                    memory: 0,
                    info: "".to_string(),
                });
                continue;
            }
        };
        let run_time = run_start.elapsed().as_micros();

        if !run_status.success() {
            cases.push(Case {
                id,
                result: Result::RuntimeError,
                time: run_time,
                memory: 0,
                info: "".to_string(),
            });
            continue;
        }

        if match config.problems[problem_id].r#type.as_str() {
            "standard" => standart_compare(&i.answer_file, &"./TMPDIR/out".to_string()),
            "strict" => strict_compare(&i.answer_file, &"./TMPDIR/out".to_string()),
            _ => unreachable!(),
        } {
            cases.push(Case {
                id,
                result: Result::Accepted,
                time: run_time,
                memory: 0,
                info: "".to_string(),
            });
            if packing.is_empty() {
                scores += i.score;
            } else {
                pack_score[packing_id] += i.score;
                if packing_index == packing[packing_id].len() as u32 - 1 {
                    scores += pack_score[packing_id];
                }
            }
        } else {
            cases.push(Case {
                id,
                result: Result::WrongAnswer,
                time: run_time,
                memory: 0,
                info: "".to_string(),
            })
        }
        fs::remove_file("./TMPDIR/out").unwrap();
    }
    return scores;
}

fn standart_compare(answer_path: &String, out_path: &String) -> bool {
    let out: Vec<_> = BufReader::new(fs::File::open(out_path).unwrap())
        .lines()
        .map(|x| x.unwrap())
        .collect();
    let answer: Vec<_> = BufReader::new(fs::File::open(answer_path).unwrap())
        .lines()
        .map(|x| x.unwrap())
        .collect();
    if out.len() != answer.len() {
        return false;
    }
    for i in 0..out.len() {
        if out[i].trim_end() != answer[i].trim_end() {
            return false;
        }
    }
    true
}

fn strict_compare(answer_path: &String, out_path: &String) -> bool {
    let answer: String = fs::read_to_string(answer_path).unwrap();
    let res: String = fs::read_to_string(out_path).unwrap();
    res == answer
}
