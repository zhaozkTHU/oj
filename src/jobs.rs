use actix_web::{post, web, HttpResponse, Responder};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Deserialize, Serialize)]
struct PostJob {
    source_code: String,
    language: String,
    user_id: u32,
    contest_id: u32,
    problem_id: u32,
}

#[derive(Deserialize, Serialize)]
struct Response {
    id: u32,
    created_time: String,
    updated_time: String,
    submission: Submission,
    state: String,
    result: Result,
    score: f32,
    cases: Vec<Case>,
}

#[derive(Deserialize, Serialize)]
struct Submission {
    source_code: String,
    language: String,
    user_id: u32,
    contest_id: u32,
    problem_id: u32,
}

#[derive(Deserialize, Serialize)]
pub struct Case {
    pub id: u32,
    pub result: Result,
    pub time: u128,
    pub memory: u32,
    pub info: String,
}

#[derive(Deserialize, Serialize)]
struct Error {
    reason: String,
    code: u8,
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum Result {
    #[serde(rename = "Compilation Success")]
    CompilationSuccess,

    #[serde(rename = "Compilation Error")]
    CompilationError,

    Accepted,

    #[serde(rename = "Wrong Answer")]
    WrongAnswer,

    #[serde(rename = "Runtime Error")]
    RuntimeError,

    #[serde(rename = "Time Limit Exceeded")]
    TimeLimitExceeded,

    Waiting,
}

#[post("/jobs")]
async fn post_jobs(body: web::Json<PostJob>, config: web::Data<Config>) -> impl Responder {
    let created_time: String = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    if !config.languages.iter().any(|x| x.name == body.language) {
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
        });
    }
    let mut index: usize = 0;
    for i in config.problems.iter().enumerate() {
        if i.1.id == body.problem_id {
            index = i.0;
        }
    }
    if !config.problems.iter().any(|x| x.id == body.problem_id) {
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
        });
    }

    let res = crate::judger::judger(&body.source_code, index, &body.language, &config);
    let updated_time: String = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let mut result: Result;
    if res.0[0].result == Result::CompilationError {
        result = Result::CompilationError;
    } else {
        result = Result::Accepted;
        for i in res.0.iter().skip(1) {
            if i.result != Result::Accepted {
                result = i.result;
                break;
            }
        }
    }

    HttpResponse::Ok().json(Response {
        id: 0,
        created_time,
        updated_time,
        submission: Submission {
            source_code: body.source_code.clone(),
            language: body.language.clone(),
            user_id: body.user_id,
            contest_id: body.contest_id,
            problem_id: body.problem_id,
        },
        state: "Finished".to_string(),
        result,
        score: res.1,
        cases: res.0,
    })
    // TODO: User management
    // TODO: Competition function
}
