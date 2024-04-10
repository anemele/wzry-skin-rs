use colored::control::set_virtual_terminal;
use colored::Colorize;
use encoding::{all::GBK, DecoderTrap, Encoding};
use regex::Regex;
use reqwest::Result;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Hero {
    pub ename: i32,
    pub id_name: String,
    pub cname: String,
    pub title: String,
}
type HeroList = Vec<Hero>;

const API_HEROLIST: &str = "https://pvp.qq.com/web201605/js/herolist.json";

fn api_heropage(id: &str) -> String {
    format!("https://pvp.qq.com/web201605/herodetail/{id}.shtml")
}

fn api_skin_url(id: i32, sn: i32) -> String {
    format!("http://game.gtimg.cn/images/yxzj/img201606/skin/hero-info/{id}/{id}-bigskin-{sn}.jpg")
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    // 1. 请求 API 获取英雄列表数据
    // 由于该列表数据不全面，仅通过它获取每个英雄的：
    // 编号（ename）、中文名（cname）、称号（title）
    let hero_list = client
        .get(API_HEROLIST)
        .send()
        .await?
        .json::<HeroList>()
        .await?;

    // 2. 创建图片保存目录
    // 初始化 css 选择器和正则表达式
    let root = Path::new("image");
    if !root.exists() {
        let _ = fs::create_dir(root);
    }
    // 此处使用 unwrap 由开发者保证合法性
    let css = Selector::parse("div.pic-pf>ul").unwrap();
    let re = Regex::new(r"(\S+?)[\s(?:&\d+)\|]+").unwrap();

    #[cfg(target_family = "windows")]
    {
        if set_virtual_terminal(true).is_err() {
            eprintln!("failed to print colorfully.")
        };
    }
    // 3. 请求每个英雄的页面，获取皮肤数据
    for hero in hero_list {
        // 首先创建本地目录
        let hero_path = root.join(format!("{}_{}", hero.cname, hero.title));
        if !hero_path.exists() {
            let _ = fs::create_dir(&hero_path);
        }
        // 请求英雄的页面
        let heropage_url = api_heropage(&hero.id_name);
        let body = client.get(&heropage_url).send().await?.bytes().await?;
        // 该网站使用 GBK 编码
        let Ok(body) = GBK.decode(&body, DecoderTrap::Strict) else {
            eprintln!(
                "网页解码失败，英雄为 {}，链接为 {}",
                hero.cname, heropage_url
            );
            continue;
        };
        let doc = Html::parse_document(&body);
        let Some(elem) = doc.select(&css).next() else {
            eprintln!(
                "css 选择器定位失败，英雄为 {}，链接为 {}",
                hero.cname, heropage_url
            );
            continue;
        };
        let Some(skins) = elem.value().attr("data-imgname") else {
            eprintln!(
                "皮肤提取失败，英雄为 {}，链接为 {}",
                hero.cname, heropage_url
            );
            continue;
        };
        // 使用正则解析皮肤数据
        let mut count_all = 0;
        let mut count_skip = 0;
        let mut count_succ = 0;
        for (i, cap) in re.captures_iter(skins).enumerate() {
            count_all += 1;
            let skin = cap[1].trim();
            let name = format!("{}_{}.jpg", i + 1, skin);
            let path = hero_path.join(&name);
            // 如果该文件已经存在，则跳过下载
            if path.exists() {
                count_skip += 1;
                continue;
            }
            let url = api_skin_url(hero.ename, i as i32 + 1);
            // 下载皮肤图片文件
            let response = client.get(url).send().await?.bytes().await?;
            if let Err(e) = tokio::fs::write(path, &response).await {
                eprintln!("{}", e)
            } else {
                count_succ += 1;
            };
        }
        let count_fail = count_all - count_skip - count_succ;
        // 输出结果
        println!(
            "{}_{}  {}/{}/{}/{}",
            hero.title,
            hero.cname,
            count_succ.to_string().green(),
            count_fail.to_string().red(),
            count_skip.to_string().yellow(),
            count_all,
        );
    }

    Ok(())
}
