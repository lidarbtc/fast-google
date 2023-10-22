use actix_web::{
    get,
    web::{self},
    App, HttpResponse, HttpServer,
};
use num_cpus;
use reqwest::{self, header};
use scraper::{Html, Selector};
use serde::Serialize;

#[derive(Serialize)]
struct GoogleResult {
    title: String,
    url: String,
    description: String,
}

async fn fetch_google(
    lang: String,
    page: String,
    query: String,
) -> Result<Vec<GoogleResult>, reqwest::Error> {
    let client = reqwest::Client::builder()
        .default_headers({
            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::USER_AGENT,
                "Mozilla/5.0 (Windows NT 10.0; rv:109.0) Gecko/20100101 Firefox/115.0"
                    .parse()
                    .unwrap(),
            );
            headers
        })
        .build()
        .unwrap();

    let res = client
        .get(format!(
            "https://www.google.com/search?q={}&start={}&hl={}&lr=lang_{}&num=20",
            query, page, lang, lang
        ))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let result = parse_google(res).await;

    Ok(result)
}

async fn parse_google(body: String) -> Vec<GoogleResult> {
    let fragment = Html::parse_document(&body);

    let results_selector = Selector::parse(".g").unwrap();
    let title_selector = Selector::parse("h3").unwrap();
    let url_selector = Selector::parse("a[href]").unwrap();
    let description_selector = Selector::parse(".VwiC3b").unwrap();

    fragment
        .select(&results_selector)
        .map(|result| {
            let title_element = result.select(&title_selector).next();
            let url_element = result.select(&url_selector).next();
            let description_element = result.select(&description_selector).next();

            GoogleResult {
                title: title_element.map_or(String::new(), |e| e.text().collect()),
                url: url_element
                    .and_then(|e| e.value().attr("href"))
                    .unwrap_or("")
                    .to_string(),
                description: description_element.map_or(String::new(), |e| e.text().collect()),
            }
        })
        .collect()
}

#[get("/{lang}/{page}/{query}")]
async fn get_google(path: web::Path<(String, String, String)>) -> HttpResponse {
    let (lang, page, query) = path.into_inner();

    match fetch_google(lang, page, query).await {
        Ok(body) => HttpResponse::Ok()
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("Access-Control-Allow-Headers", "*"))
            .append_header(("Access-Control-Allow-Methods", "*"))
            .append_header(("Access-Control-Max-Age", "1728000"))
            .json(body),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let num_workers = num_cpus::get();

    HttpServer::new(move || App::new().service(get_google))
        .workers(num_workers)
        .bind(("127.0.0.1", 6565))?
        .run()
        .await
}
