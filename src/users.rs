use actix_web::{get, post, web, HttpResponse, Responder};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Serialize, Clone)]
pub struct User {
    pub id: Option<u32>,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
struct Error {
    reason: String,
    code: u8,
    message: String,
}

lazy_static! {
    pub static ref USER_LIST: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(vec![User {
        id: Some(0),
        name: "root".to_string()
    }]));
}

#[post("/users")]
async fn post_user(user: web::Json<User>) -> impl Responder {
    let mut lock = USER_LIST.lock().unwrap();
    if let Some(id) = user.id {
        if let Ok(user_self) = lock.binary_search_by_key(&id, |x| x.id.unwrap()) {
            for (i, j) in lock.iter().enumerate() {
                if i == user_self {
                    continue;
                }
                if j.name == user.name {
                    drop(lock);
                    return HttpResponse::BadRequest().json(Error {
                        reason: "ERR_INVALID_ARGUMENT".to_string(),
                        code: 1,
                        message: format!("User name '{}' already exists.", user.name).to_string(),
                    });
                }
            }
            lock[user_self].name = user.name.clone();
            drop(lock);
            return HttpResponse::Ok().json(User {
                id: Some(id),
                name: user.name.clone(),
            });
        } else {
            drop(lock);
            return HttpResponse::NotFound().json(Error {
                reason: "ERR_NOT_FOUND".to_string(),
                code: 3,
                message: format!("User {} not found.", id).to_string(),
            });
        }
    } else {
        for i in lock.iter() {
            if i.name == user.name {
                drop(lock);
                return HttpResponse::BadRequest().json(Error {
                    reason: "ERR_INVALID_ARGUMENT".to_string(),
                    code: 1,
                    message: format!("User name '{}' already exists.", user.name).to_string(),
                });
            }
        }
        let id = Some(lock.last().unwrap().id.unwrap() + 1);
        lock.push(User {
            id,
            name: user.name.clone(),
        });
        drop(lock);
        return HttpResponse::Ok().json(User {
            id,
            name: user.name.clone(),
        });
    }
}

#[get("/users")]
async fn get_user() -> impl Responder {
    let lock = USER_LIST.lock().unwrap();
    let res = lock.clone();
    HttpResponse::Ok().json(res)
}
