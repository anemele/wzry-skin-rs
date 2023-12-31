pub const API_HEROLIST: &str = "https://pvp.qq.com/web201605/js/herolist.json";

pub fn api_heropage(id: i32) -> String {
    format!("https://pvp.qq.com/web201605/herodetail/{id}.shtml")
}

pub fn api_skin_url(id: i32, sn: i32) -> String {
    format!("http://game.gtimg.cn/images/yxzj/img201606/skin/hero-info/{id}/{id}-bigskin-{sn}.jpg")
}
