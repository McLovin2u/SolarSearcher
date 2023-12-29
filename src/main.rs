use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::str::FromStr;
use chumsky::chain::Chain;
use scraper::Html;
use chumsky::prelude::*;
use reqwest::blocking::Client;
use reqwest::header;
use regex::Regex;
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};


const BASEURL: &str = "https://www.secondsol.com";

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[command(subcommand)]
    cmd: SubCommand,
}
#[derive(Subcommand, Debug, Clone)]
enum SubCommand {
    /// Filter current Database using zipcode filters from config.txt
    Filter,

    /// Delete current Database
    ClearLocal,

    /// Check all pages for articles, update Database, delete articles that are not longer available.
    PullAll,

    /// Check specified number pages for most recent articles, update Database,  DOES NOT delete articles that are not longer available.
    Pull {
        /// Number of most recent pages to check
        #[arg(short, long, default_value_t = 1)]
        pages: usize,
    },
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Article {
    url: String,
    price_per_panel: f64,
    price_per_watt: f64,
    number_available: i32,
    min_number: i32,
    zipcode: usize,
}

#[derive(Serialize, Deserialize)]
struct Config {
    cookie: String,
    zipcodes: Vec<(usize, usize)>
}

fn load_config_file(path: String) -> Config{
    let mut file = OpenOptions::new().write(false).read(true).truncate(false).create(false).open(path).expect("No config file specified. Should be `config.txt`");
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let mut config: Config;
    config = serde_json::from_str(&buf).expect("Invalid Config File");
    println!("Config Loaded");
    config
}
fn build_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::COOKIE, "ebizuid_ebiztrader_hash=1%3Baebcf0c751b8258279b98a3b3b52d8652deb48e0c866c3de6b8acae290fd67fa6dbaa024ba2692e0a1e706792e62410951d0fcbabc1f956491688484f0739c47; ebizuid_ebiztrader_uid=64030; PHPSESSID=pm58s817bnk2i6stm55fk1bjtr; ebiztrader=g5moefkheb9n25eb8rtit8s6fd; cookies_consent=1".parse().unwrap());
    let client = reqwest::blocking::Client::builder()
        .build()
        .unwrap();
    client
}

fn filter_articles(mut articles: HashMap<usize, Article>, zipcodes: Vec<(usize, usize)>) -> Vec<Article>{
    let mut allowed: HashMap<usize, Article> = HashMap::new();
    for (lower, upper) in zipcodes {
        let passed: HashMap<usize, Article> = articles.clone().into_iter().filter(|(_id, article)| lower <= article.zipcode && article.zipcode <= upper).collect();
        for (key, value) in passed {
            allowed.insert(key, value);
        }
    }
    let mut near = allowed;
    let mut sorted_art = near.into_values().collect::<Vec<Article>>();
    sorted_art.sort_unstable_by(|x, y| x.price_per_watt.partial_cmp(&y.price_per_watt).unwrap());
    sorted_art
}

fn main() {
    let args = Args::parse();


    let config = load_config_file("config.txt".to_string());
    let client = build_client();
    let mut headers = header::HeaderMap::new();
    headers.insert(header::COOKIE, config.cookie.parse().unwrap());

    //Load articles Map
    let mut articles_file = OpenOptions::new().write(true).read(true).truncate(false).create(true).open("cached_articles").unwrap();
    let mut articles_buf = String::new();
    articles_file.read_to_string(&mut articles_buf).unwrap();
    let mut articles: HashMap<usize, Article>= HashMap::new();
    articles = serde_json::from_str(&articles_buf).unwrap_or(HashMap::new());
    println!("{} Old Articles Loaded!", articles.len());

    match args.cmd {
        SubCommand::Filter => { show_filtered(articles, config) }
        SubCommand::ClearLocal => { clear_local(articles_file) }
        SubCommand::PullAll => { pull_all(client.clone(), headers.clone(), articles_file); }
        SubCommand::Pull { pages } => { pull_pages(pages, client.clone(), headers.clone(), &mut articles, articles_file); }
    };
}

fn show_filtered(articles: HashMap<usize, Article>, config: Config) {
    let filtered= filter_articles(articles.clone(), config.zipcodes);
    println!("{} Articles meet filter", filtered.len());
    println!("Filtered Articles:");
    for article in filtered {
        println!("{}",article.url)
    }
}

fn clear_local(mut articles_file: File) {
    articles_file.set_len(0).unwrap();
    articles_file.seek(SeekFrom::Start(0)).unwrap();
}

fn pull_pages(pages: usize, client: Client, mut headers: header::HeaderMap, articles: &mut HashMap<usize, Article>, mut articles_file: File) {
    println!("Fetching new Articles, Checking {pages} pages");
    let latest_ids = fetch_latest_ids(&client, &mut headers, pages);
    println!("{} Latest IDs Found!", latest_ids.len());
    let new_ids: Vec<usize> = latest_ids.into_iter().filter(|x| !articles.contains_key(x)).collect();
    println!("{} New Articles Found!", new_ids.len());
    let new_articles = build_articles(new_ids, client, headers);
    for (key, value) in new_articles {
        articles.insert(key, value);
    }
    // Write new state of DB to File
    articles_file.set_len(0).unwrap();
    articles_file.seek(SeekFrom::Start(0)).unwrap();
    articles_file.write_all(serde_json::to_string(&articles).unwrap().as_ref()).unwrap();
}

fn pull_all(client: Client, mut headers: header::HeaderMap, mut articles_file: File) {
    println!("Updating all Articles, Checking all pages");
    let latest_ids = fetch_latest_ids(&client, &mut headers, 93);
    println!("{} Articles Found!", latest_ids.len());
    let new_articles = build_articles(latest_ids, client, headers);
    // Write new state of DB to File
    articles_file.set_len(0).unwrap();
    articles_file.seek(SeekFrom::Start(0)).unwrap();
    articles_file.write_all(serde_json::to_string(&new_articles).unwrap().as_ref()).unwrap();
}

fn build_articles(links: Vec<usize>, client: Client, headers: header::HeaderMap) -> HashMap<usize, Article> {
    let mut articles: HashMap<usize, Article> = HashMap::new();
    let dummy_article = Article{
        url: "".to_string(),
        price_per_panel: 0.0,
        price_per_watt: 99999.00,
        number_available: 0,
        min_number: 0,
        zipcode: 0,
    };
    for id in links {
        println!("Bulding Aricle {id}");
        let fullurl = format!("{BASEURL}/de/anzeige/{id}");
        let html = fetch_html(fullurl.clone(), client.clone(), headers.clone());
        let article = parse_article(html);
        if article.is_some() {
            articles.insert(id, article.unwrap());
        } else {
            articles.insert(id, dummy_article.clone());
        }
    }
    articles
}

fn fetch_latest_ids(client: &Client, mut headers: &mut header::HeaderMap, page_count: usize) -> Vec<usize> {
    let links = (1..=page_count).map(|page| get_links_from_page(page, client.clone(), headers.clone())).flatten().collect();
    links
}

fn fetch_html(fullurl: String, client: Client, headers: header::HeaderMap) -> String {
    let res = client.get(fullurl)
        .headers(headers)
        .send().unwrap()
        .text().unwrap();
    // Html::parse_document(&res)
    res
}


fn get_links_from_page(page: usize, client: Client, headers: header::HeaderMap) -> Vec<usize> {
    println!("Scraping Page {page}");
    let html = fetch_html(format!("https://www.secondsol.com/de/marktplatzfilter/?kat2=40939&kategorie=17&sortierung=alter&currentpage={page}&level3=false"), client, headers);
    let html = Html::parse_document(&html);
    let selector = &scraper::Selector::parse(".articlebox-new").unwrap();
    let mut articles = html.select(&selector);
    let mut article_ids = vec![];
    for article in articles {
        let url = article
            .select(&scraper::Selector::parse("a").unwrap())
            .next()
            .and_then(|a| a.value().attr("href"))
            .map(|str| str.to_owned());
        if url.is_some(){
            article_ids.push(url.unwrap());
        }
    }
    article_ids.iter().map(to_id).collect()
}
fn to_id(url: &String) -> usize {
    let mut re = Regex::new(r"[0-9]+").unwrap();
    re.find(url).unwrap().as_str().parse::<usize>().unwrap()
}
fn parse_article(html_string: String) -> Option<Article> {
    let html = Html::parse_document(&html_string);
    let article_id = html
        .select(&scraper::Selector::parse("table.table-product").unwrap())
        .next().expect("Did not find table").select(&scraper::Selector::parse("td").unwrap())
        .next()
        .map(|a| a.text().collect::<String>());
    let price_per_panel = html
        .select(&scraper::Selector::parse("span.priceDecoAlpha").unwrap())
        .next()
        .map(|a| a.text().collect::<String>());
    let price_per_watt = html
        .select(&scraper::Selector::parse("span.priceDecoBeta").unwrap())
        .next()
        .map(|a| a.text().collect::<String>());
    let location = html
        .select(&scraper::Selector::parse("#location").unwrap())
        .next()
        .map(|a| a.text().collect::<String>());
    Some(Article { url: parse_url(&article_id?),  price_per_panel: parse_price_per_panel(price_per_panel?), price_per_watt: parse_price_per_watt(price_per_watt?), number_available: 0, min_number: 0, zipcode: parse_zipcode(&location?)?.to_owned() })
}

fn parse_price_per_panel(string: String) -> f64 {
    let cleaned = string.trim().replace(".","").trim_start_matches("ab: ").trim_end_matches("  EUR  / Stk").trim_end_matches("  EUR/Stk").trim_end_matches("  CHF  / Stk").replace(",", ".");
    f64::from_str(&*cleaned).expect(&*format!("Could not parse{}", cleaned))
}
fn parse_price_per_watt(string: String) -> f64 {
    let cleaned = string.trim().trim_end_matches("  EUR  / Wp").trim_end_matches("  EUR / Wp").trim_end_matches("  CHF  / Wp").replace(",", ".");
    f64::from_str(&*cleaned).unwrap_or(420.0)
}

fn parse_url(string: &str) -> String {
    let re = Regex::new(r"[0-9]+").unwrap();
    let uid = re.find(string).unwrap();
    format!("{BASEURL}/de/anzeige/{}",uid.as_str())
}

fn parse_zipcode(string: &str) -> Option<usize> {
    let re = Regex::new(r"[0-9][0-9][0-9][0-9][0-9]").unwrap();
    Some(re.find(string)?.as_str().parse().unwrap())
}