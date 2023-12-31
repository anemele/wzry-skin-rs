use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use futures::{stream, StreamExt};
use regex::Regex;
use reqwest::{Client, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::{Path, PathBuf};
use std::{fs, io::Write};

mod api;
use api::*;

#[derive(Debug, Serialize, Deserialize)]
struct Hero<'a> {
    pub ename: i32,
    pub cname: &'a str,
    pub title: &'a str,
}

const SAVE_ROOT: &str = "image";
const CONCURRENT_REQUESTS: usize = 8;

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    let body = client.get(API_HEROLIST).send().await?.text().await?;

    let hero_list: Vec<Hero> = match serde_json::from_str(&body) {
        Ok(hero) => hero,
        Err(e) => panic!("{}", e),
    };

    let root = Path::new(SAVE_ROOT);
    if !root.exists() {
        let _ = fs::create_dir(root);
    }

    let css = Selector::parse("div.pic-pf>ul").unwrap();
    let re = Regex::new(r"(\S+?)[\s(?:&\d+)\|]+").unwrap();

    let skins_list = stream::iter(hero_list)
        .map(|hero| {
            let client = &client;
            let css = &css;
            let re = &re;
            async move {
                let hero_path = root.join(format!("{}_{}", hero.cname, hero.title));
                if !hero_path.exists() {
                    let _ = fs::create_dir(&hero_path);
                }

                let mut url_list = Vec::<(PathBuf, String)>::new();
                if let Ok(skins) = get_heropage(&client, &css, &hero).await {
                    for (i, cap) in re.captures_iter(skins.as_str()).enumerate() {
                        // println!("{cap:?}")
                        // println!("{}", cap[1].trim())
                        // println!("{i}");
                        let skin = cap[1].trim();
                        // println!("{skin}")

                        let skin_file_name = format!("{}_{}.jpg", i + 1, skin);
                        let save_path = hero_path.join(&skin_file_name);
                        let skin_url = api_skin_url(hero.ename, i as i32 + 1);
                        url_list.push((save_path, skin_url))
                    }
                }

                url_list
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    skins_list
        .for_each(|list| async {
            for (path, url) in list {
                if path.exists() {
                    continue;
                }
                let client = &client;

                (async {
                    match get_skinimag(&client, &url, &path).await {
                        Ok(_) => println!("done: {}", path.display()),
                        Err(e) => eprintln!("{}", e),
                    }
                })
                .await
            }
        })
        .await;

    Ok(())
}

async fn get_heropage<'a>(client: &Client, css: &Selector, hero: &Hero<'a>) -> Result<String> {
    let heropage_url = api_heropage(hero.ename);
    let response = client.get(heropage_url).send().await?.bytes().await?;
    // println!("{response}");
    let body = GBK.decode(&response, DecoderTrap::Strict).unwrap();

    let document = Html::parse_document(&body);
    let skins = document
        .select(&css)
        .next()
        .unwrap()
        .value()
        .attr("data-imgname")
        .unwrap();
    // println!("{skins}");

    Ok(skins.to_string())
}

async fn get_skinimag(client: &Client, url: &str, save_path: &Path) -> Result<()> {
    let response = client.get(url).send().await?.bytes().await?;
    let _ = fs::File::create(save_path).unwrap().write(&response);
    Ok(())
}
