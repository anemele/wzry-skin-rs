use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use futures::{stream, StreamExt};
use regex::Regex;
use reqwest::{Client, Result};
use scraper::{Html, Selector};
use std::path::Path;
use std::{fs, io::Write};

mod api;
use api::*;

mod types;
use types::*;

const SAVE_ROOT: &str = "image";
const CONCURRENT_REQUESTS: usize = 8;

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
    let skins_list = stream::iter(hero_list)
        .map(|hero| {
            let hero_path = root.join(format!("{}_{}", hero.cname, hero.title));
            if !hero_path.exists() {
                let _ = fs::create_dir(&hero_path);
            }
            let client = &client;
            let css = &css;
            async move {
                (
                    hero_path,
                    hero.ename,
                    match get_heropage(&client, &css, &hero).await {
                        Ok(skins) => skins,
                        Err(_) => "".to_string(),
                    },
                )
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    // Step 4
    skins_list
        .for_each(|(hero_path, hero_id, skins)| {
            let client = &client;
            let re = &re;

            async move {
                for (i, cap) in re.captures_iter(skins.as_str()).enumerate() {
                    let skin = cap[1].trim();
                    let name = format!("{}_{}.jpg", i + 1, skin);
                    let path = hero_path.join(&name);
                    if path.exists() {
                        continue;
                    }
                    let url = api_skin_url(hero_id, i as i32 + 1);
                    (async {
                        match get_skinimag(&client, &url, &path).await {
                            Ok(_) => println!("done: {}", path.display()),
                            Err(e) => eprintln!("{}", e),
                        }
                    })
                    .await
                }
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
