use actix_web::{get, post, web, HttpResponse, Responder};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::{cmp::Ordering, vec};

use crate::jobs::{Result, RESPONSE_LIST};
use crate::users::USER_LIST;
use crate::{config::Config, users::User};

#[derive(Deserialize)]
struct Info {
    scoring_rule: Option<String>,
    tie_breaker: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Error {
    reason: String,
    code: u8,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Contest {
    pub id: usize,
    pub name: String,
    pub from: String,
    pub to: String,
    pub problem_ids: Vec<usize>,
    pub user_ids: Vec<usize>,
    pub submission_limit: u32,
}

#[derive(Deserialize, Clone)]
struct PostContest {
    id: Option<usize>,
    name: String,
    from: String,
    to: String,
    problem_ids: Vec<usize>,
    user_ids: Vec<usize>,
    submission_limit: u32,
}

lazy_static! {
    pub static ref CONTEST_LIST: Arc<Mutex<Vec<Contest>>> = Arc::new(Mutex::new(vec![Contest {
        id: 0,
        name: String::new(),
        from: String::new(),
        to: String::new(),
        problem_ids: Vec::new(),
        user_ids: Vec::new(),
        submission_limit: 0,
    }]));
}

#[post("/contests")]
async fn post_contest(body: web::Json<PostContest>, config: web::Data<Config>) -> impl Responder {
    let mut body = body.clone();
    body.problem_ids.sort();
    body.user_ids.sort();

    let mut contest_list = CONTEST_LIST.lock().unwrap();

    if *body.problem_ids.iter().max().unwrap() >= config.problems.len() {
        drop(contest_list);
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: "Problem id not found".to_string(),
        });
    }
    if body.id.is_some() && body.id.unwrap() == 0 {
        drop(contest_list);
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: "Contest id should not be 0.".to_string(),
        });
    }
    let user_list = USER_LIST.lock().unwrap();
    if *body.user_ids.iter().max().unwrap() >= user_list.len() {
        drop(user_list);
        drop(contest_list);
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: "User id not found".to_string(),
        });
    }
    drop(user_list);

    if body.id.is_none() {
        let id = contest_list.len();
        contest_list.push(Contest {
            id,
            name: body.name.clone(),
            from: body.from.clone(),
            to: body.to.to_string(),
            problem_ids: body.problem_ids.clone(),
            user_ids: body.user_ids.clone(),
            submission_limit: body.submission_limit,
        });
        drop(contest_list);
        return HttpResponse::Ok().json(Contest {
            id,
            name: body.name.clone(),
            from: body.from.clone(),
            to: body.to.to_string(),
            problem_ids: body.problem_ids.clone(),
            user_ids: body.user_ids.clone(),
            submission_limit: body.submission_limit,
        });
    } else {
        if body.id.unwrap() >= contest_list.len() {
            drop(contest_list);
            return HttpResponse::NotFound().json(Error {
                reason: "ERR_NOT_FOUND".to_string(),
                code: 3,
                message: format!("Contest {} not found", body.id.unwrap()).to_string(),
            });
        } else {
            contest_list[body.id.unwrap()] = Contest {
                id: body.id.unwrap(),
                name: body.name.clone(),
                from: body.from.clone(),
                to: body.to.clone(),
                problem_ids: body.problem_ids.clone(),
                user_ids: body.user_ids.clone(),
                submission_limit: body.submission_limit,
            };
            drop(contest_list);
            return HttpResponse::Ok().json(Contest {
                id: body.id.unwrap(),
                name: body.name.clone(),
                from: body.from.clone(),
                to: body.to.clone(),
                problem_ids: body.problem_ids.clone(),
                user_ids: body.user_ids.clone(),
                submission_limit: body.submission_limit,
            });
        }
    }
}

#[get("/contests")]
async fn get_contests() -> impl Responder {
    let contest_list = CONTEST_LIST.lock().unwrap();
    let res = HttpResponse::Ok().json(
        contest_list
            .clone()
            .into_iter()
            .skip(1)
            .collect::<Vec<Contest>>(),
    );
    drop(contest_list);
    res
}

#[get("/contests/{contest_id}")]
async fn get_contests_by_id(contest_id: web::Path<usize>) -> impl Responder {
    let contest_list = CONTEST_LIST.lock().unwrap();
    if contest_id.clone() >= contest_list.len() {
        drop(contest_list);
        return HttpResponse::NotFound().json(Error {
            reason: "ERR_NOT_FOUND".to_string(),
            code: 3,
            message: format!("Contest {} not found.", contest_id.clone()),
        });
    } else {
        let res = contest_list[contest_id.clone()].clone();
        drop(contest_list);
        return HttpResponse::Ok().json(res);
    }
}

#[get("/contests/{contest_id}/ranklist")]
async fn get_contests_ranklist(
    contest_id: web::Path<u32>,
    info: web::Query<Info>,
    config: web::Data<Config>,
) -> impl Responder {
    let problems_count = config.problems.len();

    let lock = crate::users::USER_LIST.lock().unwrap();
    let users = lock.clone();
    drop(lock);

    let mut users_score = vec![vec![(0 as f32, String::new()); problems_count]; users.len()];
    let mut submission_count = vec![0; users.len()];
    let mut latest_time = vec![String::new(); users.len()];

    let lock = crate::jobs::RESPONSE_LIST.lock().unwrap();
    let mut response_list = lock.clone();
    drop(lock);

    if contest_id.clone() != 0 {
        response_list = response_list
            .into_iter()
            .filter(|x| x.submission.contest_id == contest_id.clone())
            .collect();
    }

    for i in response_list.iter_mut() {
        if config.problems[i.submission.problem_id as usize].r#type == "dynamic_ranking" {
            if let Some(ratio) = config.problems[i.submission.problem_id as usize]
                .misc
                .dynamic_ranking_ratio
                .as_ref()
            {
                let mut new_score: f32 = 0.0;
                if i.result == Result::Accepted {
                    let mut shortest = vec![0 as u128; i.score_vec.len()];
                    let lock = RESPONSE_LIST.lock().unwrap();
                    for i in lock.iter() {
                        if i.submission.problem_id == i.submission.problem_id as u32 {
                            for j in i.cases.iter().enumerate().skip(1) {
                                if shortest[j.0 - 1] == 0 || j.1.time < shortest[j.0 - 1] {
                                    shortest[j.0 - 1] = j.1.time;
                                }
                            }
                        }
                    }
                    for (u, s) in i.score_vec.iter().enumerate() {
                        new_score += *s
                            * (1 as f32 - *ratio
                                + *ratio * (shortest[u] as f32) / (i.cases[u + 1].time as f32));
                    }
                    drop(lock);
                } else {
                    new_score = i.score * ratio;
                }
                i.score = new_score;
            }
        }
    }

    for i in response_list.iter() {
        if contest_id.clone() != 0 {
            if i.submission.contest_id != contest_id.clone() {
                continue;
            }
        }
        submission_count[i.submission.user_id as usize] += 1;
        if info.scoring_rule.is_some() && info.scoring_rule.clone().unwrap().as_str() == "highest" {
            if i.score
                > users_score[i.submission.user_id as usize][i.submission.problem_id as usize].0
            {
                users_score[i.submission.user_id as usize][i.submission.problem_id as usize] =
                    (i.score, i.created_time.clone());
            }
        } else {
            if i.created_time
                > users_score[i.submission.user_id as usize][i.submission.problem_id as usize].1
                || users_score[i.submission.user_id as usize][i.submission.problem_id as usize]
                    .1
                    .is_empty()
            {
                users_score[i.submission.user_id as usize][i.submission.problem_id as usize] =
                    (i.score, i.created_time.clone());
            }
        }
    }

    let mut total_score = vec![(0 as f32, 0 as usize); users.len()];
    for (i, j) in users_score.iter().enumerate() {
        let mut score = 0.0 as f32;
        for x in j.iter() {
            score += x.0;
        }
        total_score[i] = (score, i);
    }

    match info.tie_breaker.as_deref() {
        Some("submission_count") => {
            total_score.sort_by(|a, b| {
                if a.0 == b.0 {
                    submission_count[a.1].cmp(&submission_count[b.1])
                } else {
                    b.0.partial_cmp(&a.0).unwrap()
                }
            });
        }
        Some("submission_time") => {
            for (i, j) in users_score.iter().enumerate() {
                for a in j.iter() {
                    if latest_time[i].is_empty() || a.1 > latest_time[i] {
                        latest_time[i] = a.1.clone();
                    }
                }
            }
            total_score.sort_by(|a, b| {
                if a.0 == b.0 {
                    if !latest_time[a.1].is_empty() && !latest_time[b.1].is_empty() {
                        latest_time[a.1].cmp(&latest_time[b.1])
                    } else if latest_time[a.1].is_empty() && latest_time[a.1].is_empty() {
                        Ordering::Equal
                    } else if latest_time[a.1].is_empty() {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                } else {
                    b.0.partial_cmp(&a.0).unwrap()
                }
            });
        }
        _ => total_score.sort_by(|a, b| {
            if a.0 == b.0 {
                a.1.cmp(&b.1)
            } else {
                b.0.partial_cmp(&a.0).unwrap()
            }
        }),
    }
    #[derive(Serialize)]
    struct Res {
        user: User,
        rank: u32,
        scores: Vec<f32>,
        submission_count: u32,
    }
    let mut res: Vec<Res> = Vec::new();

    let mut last_score = 0 as f32;
    let mut last_id = 0;
    let mut rank = 1;
    let mut totals = 0;

    for j in total_score.iter() {
        match info.tie_breaker.as_deref() {
            Some("submission_count") => {
                if (j.0 != last_score || submission_count[j.1] != submission_count[last_id])
                    && totals != 0
                {
                    rank = totals + 1;
                }
            }
            Some("submission_time") => {
                if (j.0 != last_score || latest_time[j.1] != latest_time[last_id]) && totals != 0 {
                    rank = totals + 1;
                }
            }
            Some("user_id") => {
                if totals != 0 {
                    rank = totals + 1;
                }
            }
            _ => {
                if j.0 != last_score && totals != 0 {
                    rank = totals + 1;
                }
            }
        }
        last_score = j.0;
        last_id = j.1;
        res.push(Res {
            user: users[j.1].clone(),
            rank,
            scores: users_score[j.1].iter().map(|x| x.0).collect(),
            submission_count: submission_count[j.1],
        });
        totals += 1;
    }

    if contest_id.clone() != 0 {
        let contest_list = CONTEST_LIST.lock().unwrap();
        // score of user who is not in the contest must be 0, so remove these users will not change the rank
        res = res
            .into_iter()
            .filter(|x| {
                contest_list[contest_id.clone() as usize]
                    .user_ids
                    .contains(&(x.user.id.unwrap() as usize))
            })
            .collect();
        for i in res.iter_mut() {
            let mut tmp: Vec<f32> = Vec::new();
            for j in contest_list[contest_id.clone() as usize].problem_ids.iter() {
                tmp.push(i.scores[*j]);
            }
            i.scores = tmp;
        }
    }

    HttpResponse::Ok().json(res)
}
