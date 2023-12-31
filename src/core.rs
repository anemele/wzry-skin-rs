use crate::api::{api_heropage, api_skin_url};
use crate::types::Hero;
use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use regex::Regex;
use reqwest::{Client, Result};
use scraper::{Html, Selector};
use std::path::{Path, PathBuf};

pub async fn get_heropage<'a>(client: &Client, css: &Selector, hero: &Hero<'a>) -> Result<String> {
    let heropage_url = api_heropage(hero.ename);
    let body = client.get(heropage_url).send().await?.bytes().await?;
    let body = GBK.decode(&body, DecoderTrap::Strict).unwrap();

    let document = Html::parse_document(&body);
    let skins = document
        .select(&css)
        .next()
        .unwrap()
        .value()
        .attr("data-imgname")
        .unwrap();

    Ok(skins.to_string())
}

pub async fn gen_skin_path_url(
    re: &Regex,
    path: &Path,
    id: i32,
    skins: &str,
) -> Vec<(PathBuf, String)> {
    let mut ret = vec![];
    for (i, cap) in re.captures_iter(skins).enumerate() {
        let skin = cap[1].trim();
        let name = format!("{}_{}.jpg", i + 1, skin);
        let path = path.join(&name);
        if path.exists() {
            continue;
        }
        let url = api_skin_url(id, i as i32 + 1);
        ret.push((path, url))
    }
    ret
}

pub async fn get_skinimag(client: &Client, path: &Path, url: &str) -> Result<()> {
    let response = client.get(url).send().await?.bytes().await?;
    match tokio::fs::write(path, &response).await {
        Ok(_) => println!("done: {}", path.display()),
        Err(e) => eprintln!("{}", e),
    };
    Ok(())
}
