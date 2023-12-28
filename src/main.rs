use std::path::Path;
use std::{fs, io::Write};

use encoding::all::GBK;
use encoding::{DecoderTrap, Encoding};
use regex::Regex;
use reqwest::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json;

pub const API_HEROLIST: &str = "https://pvp.qq.com/web201605/js/herolist.json";
pub const API_IP_ADDRESS: &str = "https://httpbin.org/ip";
pub const SAVE_ROOT: &str = "image";

pub fn api_heropage(id: i32) -> String {
    format!("https://pvp.qq.com/web201605/herodetail/{id}.shtml")
}

pub fn api_skin_url(id: i32, sn: i32) -> String {
    format!("http://game.gtimg.cn/images/yxzj/img201606/skin/hero-info/{id}/{id}-bigskin-{sn}.jpg")
}

#[derive(Debug, Serialize, Deserialize)]
struct Hero<'a> {
    pub ename: i32,
    pub cname: &'a str,
    pub title: &'a str,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    let body = client.get(API_HEROLIST).send().await?.text().await?;
    // println!("{body}");

    let hero_list: Vec<Hero> = match serde_json::from_str(&body) {
        Ok(hero) => hero,
        Err(e) => panic!("{}", e),
    };
    // println!("{:?}", hero_list);

    let root_path = Path::new(SAVE_ROOT);
    if !root_path.exists() {
        let _ = fs::create_dir(root_path);
    }

    let selector = Selector::parse("div.pic-pf>ul").unwrap();
    let re = Regex::new(r"(\S+?)[\s(?:&\d+)\|]+").unwrap();

    for hero in hero_list {
        // println!("{hero:?}");

        let hero_path = root_path.join(format!("{}_{}", hero.cname, hero.title));
        if !hero_path.exists() {
            let _ = fs::create_dir(&hero_path);
        }

        let heropage_url = api_heropage(hero.ename);
        let response = client.get(heropage_url).send().await?.bytes().await?;
        // println!("{response}");
        let body = GBK.decode(&response, DecoderTrap::Strict).unwrap();

        let document = Html::parse_document(&body);
        let skins = document
            .select(&selector)
            .next()
            .unwrap()
            .value()
            .attr("data-imgname")
            .unwrap();
        // println!("{skins}");

        for (i, cap) in re.captures_iter(skins).enumerate() {
            // println!("{cap:?}")
            // println!("{}", cap[1].trim())
            // println!("{i}");
            let skin = cap[1].trim();
            // println!("{skin}")

            let skin_url = api_skin_url(hero.ename, i as i32 + 1);
            let response = client.get(skin_url).send().await?.bytes().await?;

            let skin_file_name = format!("{}_{}.jpg", i + 1, skin);
            let mut file = match fs::File::create(hero_path.join(&skin_file_name)) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("fail: {e}");
                    continue;
                }
            };

            let _ = file.write(&response);
            println!("done: {skin_file_name}")
        }
    }

    Ok(())
}
