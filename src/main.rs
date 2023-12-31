use futures::{stream, StreamExt};
use regex::Regex;
use reqwest::Result;
use scraper::Selector;
use std::fs;
use std::path::Path;

mod api;
use api::*;
mod core;
use core::{gen_skin_path_url, get_heropage, get_skinimag};
mod types;
use types::*;

const SAVE_ROOT: &str = "image";

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    // Step 1, get the herolist from API
    let body = client.get(API_HEROLIST).send().await?.bytes().await?;

    let hero_list: Vec<Hero> = match serde_json::from_slice(&body) {
        Ok(list) => list,
        Err(e) => panic!("{}", e),
    };

    // Step 2, set the constants: savepath, selector, regex, ...
    let root = Path::new(SAVE_ROOT);
    if !root.exists() {
        let _ = fs::create_dir(root);
    }

    let css = Selector::parse("div.pic-pf>ul").unwrap();
    let re = Regex::new(r"(\S+?)[\s(?:&\d+)\|]+").unwrap();

    // Step 3
    stream::iter(hero_list)
        .map(|hero| {
            let client = &client;
            let css = &css;

            async move {
                let hero_path = root.join(format!("{}_{}", hero.cname, hero.title));
                if !hero_path.exists() {
                    let _ = fs::create_dir(&hero_path);
                }
                (
                    hero_path,
                    hero.ename,
                    match get_heropage(client, css, &hero).await {
                        Ok(s) => s,
                        Err(_) => "".to_string(),
                    },
                )
            }
        })
        .buffer_unordered(8)
        .map(|(path, id, skins)| {
            let re: &Regex = &re;
            async move { gen_skin_path_url(re, &path, id, &skins).await }
        })
        .buffer_unordered(8)
        .for_each(|list| async {
            for (path, url) in list {
                let client = &client;
                async move {
                    let _ = get_skinimag(client, &path, &url).await;
                }
                .await
            }
        })
        .await;

    Ok(())
}
