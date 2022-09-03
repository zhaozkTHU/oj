use actix_web::{get, post, put, web, HttpResponse, Responder};
use chrono::prelude::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::{config::Config, users::USER_LIST};

#[derive(Deserialize, Serialize)]
struct PostJob {
    source_code: String,
    language: String,
    user_id: u32,
    contest_id: u32,
    problem_id: u32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Response {
    id: u32,
    pub created_time: String,
    updated_time: String,
    pub submission: Submission,
    state: String,
    result: Result,
    pub score: f32,
    cases: Vec<Case>,
}

lazy_static! {
    pub static ref RESPONSE_LIST: Arc<Mutex<Vec<Response>>> = Arc::new(Mutex::new(Vec::new()));
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Submission {
    source_code: String,
    language: String,
    pub user_id: u32,
    pub contest_id: u32,
    pub problem_id: u32,
}

#[derive(Deserialize, Serialize, Clone)]
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
    message: String,
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

    Skipped,
}

#[derive(Deserialize)]
struct Info {
    user_id: Option<u32>,
    user_name: Option<String>,
    contest_id: Option<u32>,
    problem_id: Option<u32>,
    language: Option<String>,
    from: Option<String>,
    to: Option<String>,
    state: Option<String>,
    result: Option<Result>,
}

lazy_static! {
    static ref JOB_ID: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

#[post("/jobs")]
async fn post_jobs(body: web::Json<PostJob>, config: web::Data<Config>) -> impl Responder {
    let created_time: String = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    if !config.languages.iter().any(|x| x.name == body.language) {
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: "".to_string(),
        });
    }
    let lock = crate::users::USER_LIST.lock().unwrap();
    if lock
        .iter()
        .find(|x| x.id.unwrap() == body.user_id)
        .is_none()
    {
        drop(lock);
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: "".to_string(),
        });
    }
    drop(lock);
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
            message: "".to_string(),
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

    let mut lock = JOB_ID.lock().unwrap();
    let id = lock.clone();
    *lock += 1;
    drop(lock);

    let mut lock = RESPONSE_LIST.lock().unwrap();
    lock.push(Response {
        id,
        created_time: created_time.clone(),
        updated_time: updated_time.clone(),
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
        cases: res.0.clone(),
    });
    drop(lock);

    HttpResponse::Ok().json(Response {
        id,
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

#[get("/jobs")]
async fn get_jobs(info: web::Query<Info>) -> impl Responder {
    let lock = RESPONSE_LIST.lock().unwrap();
    let response_list = lock.clone();
    let mut filtered = vec![true; response_list.len()];
    drop(lock);

    filter(&info, &response_list, &mut filtered);

    let mut res: Vec<Response> = Vec::new();
    for (i, j) in filtered.iter().enumerate() {
        if *j {
            res.push(response_list[i].clone());
        }
    }

    res.sort_by(|a, b| {
        DateTime::parse_from_rfc3339(&a.created_time)
            .unwrap()
            .cmp(&DateTime::parse_from_rfc3339(&b.created_time).unwrap())
    });

    #[derive(Serialize)]
    struct Responses(Vec<Response>);
    HttpResponse::Ok().json(Responses(res))
}

#[get("/jobs/{jobid}")]
async fn get_jobid(jobid: web::Path<u32>) -> impl Responder {
    let jobid = jobid.clone();
    let lock = JOB_ID.lock().unwrap();
    let max_id = lock.clone();
    drop(lock);
    if jobid >= max_id {
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: format!("Job {} not found", jobid).to_string(),
        });
    }
    let lock = RESPONSE_LIST.lock().unwrap();
    let res = lock[jobid as usize].clone();
    drop(lock);
    HttpResponse::Ok().json(res)
}

#[put("/jobs/{jobid}")]
async fn put_jobid(jobid: web::Path<u32>, config: web::Data<Config>) -> impl Responder {
    let jobid = jobid.clone();
    let lock = JOB_ID.lock().unwrap();
    let max_id = lock.clone();
    drop(lock);
    if jobid >= max_id {
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: format!("Job {} not found.", jobid).to_string(),
        });
    }

    let lock = RESPONSE_LIST.lock().unwrap();
    let response = lock[jobid as usize].clone();
    drop(lock);

    if response.state.as_str() != "Finished" {
        return HttpResponse::BadRequest().json(Error {
            reason: "ERR_INVALID_STATE".to_string(),
            code: 2,
            message: format!("Job {} not finished.", jobid),
        });
    }

    let res = crate::judger::judger(
        &response.submission.source_code,
        response.submission.problem_id as usize,
        &response.submission.language,
        &config,
    );
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

    let updated_time = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut lock = RESPONSE_LIST.lock().unwrap();
    lock[jobid as usize] = Response {
        id: response.id,
        created_time: response.created_time.clone(),
        updated_time: updated_time.clone(),
        submission: response.submission.clone(),
        state: "Finished".to_string(),
        result,
        score: res.1,
        cases: res.0.clone(),
    };
    drop(lock);
    HttpResponse::Ok().json(Response {
        id: response.id,
        created_time: response.created_time,
        updated_time,
        submission: response.submission,
        state: "Finished".to_string(),
        result,
        score: res.1,
        cases: res.0,
    })
}

fn filter(info: &web::Query<Info>, response_list: &Vec<Response>, filtered: &mut Vec<bool>) {
    // TODO
    if let Some(user_id) = info.user_id.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.submission.user_id != *user_id {
                filtered[i] = false;
            }
        }
    }
    if let Some(user_name) = info.user_name.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            let lock = USER_LIST.lock().unwrap();
            if lock[j.submission.user_id as usize].name != *user_name {
                filtered[i] = false;
            }
            drop(lock);
        }
    }
    if let Some(contest_id) = info.contest_id.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.submission.contest_id != *contest_id {
                filtered[i] = false;
            }
        }
    }
    if let Some(problem_id) = info.problem_id.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.submission.problem_id != *problem_id {
                filtered[i] = false;
            }
        }
    }
    if let Some(language) = info.language.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.submission.language != *language {
                filtered[i] = false;
            }
        }
    }
    if let Some(from) = info.from.as_ref() {
        let t = DateTime::parse_from_rfc3339(from).unwrap();
        for (i, j) in response_list.iter().enumerate() {
            if DateTime::parse_from_rfc3339(&j.created_time).unwrap() < t {
                filtered[i] = false;
            }
        }
    }
    if let Some(to) = info.to.as_ref() {
        let t = DateTime::parse_from_rfc3339(to).unwrap();
        for (i, j) in response_list.iter().enumerate() {
            if DateTime::parse_from_rfc3339(&j.created_time).unwrap() > t {
                filtered[i] = false;
            }
        }
    }
    if let Some(state) = info.state.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.state != *state {
                filtered[i] = false;
            }
        }
    }
    if let Some(result) = info.result.as_ref() {
        for (i, j) in response_list.iter().enumerate() {
            if j.result != *result {
                filtered[i] = false;
            }
        }
    }
}
