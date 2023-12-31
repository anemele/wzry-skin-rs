use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Hero<'a> {
    pub ename: i32,
    pub cname: &'a str,
    pub title: &'a str,
}
