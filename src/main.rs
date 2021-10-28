use anyhow::{anyhow, Result};
use clap::Parser;
use colored::Colorize;
use mime::Mime;
use reqwest::{header, Client, Response};
use std::fmt::Debug;
use std::{collections::HashMap, str::FromStr};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use url::{ParseError, Url};

#[derive(Parser, Debug)]
struct GET {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}
#[derive(Parser, Debug)]
struct POST {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}
#[derive(Parser, Debug)]
struct PUT {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}
#[derive(Parser, Debug)]
struct PATCH {
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}

#[derive(Parser, Debug)]
enum SubCommand {
    Get(GET),
    Post(POST),
    Put(PUT),
    Patch(PATCH),
}

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Tonitr <tonitrnel@outlook.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Debug, PartialEq)]
struct KvPair {
    key: String,
    value: String,
}

impl FromStr for KvPair {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("=");
        let err = || anyhow!(format!("Failed to parse {}", s));
        Ok(Self {
            key: (split.next().ok_or_else(err)?).to_string(),
            value: (split.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_url(url: &str) -> Result<String, ParseError> {
    Url::parse(url)?;
    Ok(url.into())
}

fn parse_kv_pair(s: &str) -> Result<KvPair, anyhow::Error> {
    Ok(s.parse()?)
}

fn print_status(res: &Response) {
    print!("{}\n", "[status]".bold().truecolor(164, 111, 164));
    let version = format!("{:?}", res.version()).truecolor(67, 95, 164);
    let status = format!("{}", res.status()).truecolor(117, 157, 255);
    println!("{} {}", version, status);
}

fn print_headers(res: &Response) {
    print!("{}\n", "[headers]".bold().truecolor(164, 111, 164));
    for (name, value) in res.headers() {
        println!("{}: {:?}", name.to_string().truecolor(157, 173, 212), value)
    }
}

fn print_syntect(content: &str, ext: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let theme = &ts.themes["base16-eighties.dark"];
    let mut h = HighlightLines::new(syntax, theme);
    for line in LinesWithEndings::from(content) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        print!("{}", escaped)
    }
}

fn print_body(m: Option<Mime>, body: &String) {
    print!("{}\n", "[body]".bold().truecolor(164, 111, 164));
    match m {
        Some(v) if v == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML || v == mime::TEXT_HTML_UTF_8 => {
            print_syntect(body, "html")
        }
        Some(v) if v == mime::TEXT_CSS || v == mime::TEXT_CSS_UTF_8 => print_syntect(body, "css"),
        Some(v) if v == mime::APPLICATION_JAVASCRIPT => print_syntect(body, "javascript"),
        _ => println!("{}", body),
    }
}
fn parse_mime(res: &Response) -> Option<Mime> {
    res.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())
}

async fn print_response(res: Response) -> Result<()> {
    print_status(&res);
    print_headers(&res);
    let mime = parse_mime(&res);
    let body = res.text().await?;
    print_body(mime, &body);
    Ok(())
}

struct HttpRequest {}

impl HttpRequest {
    async fn get(client: Client, args: &GET) -> Result<()> {
        let res = client.get(&args.url).send().await?;
        Ok(print_response(res).await?)
    }
    async fn post(client: Client, args: &POST) -> Result<()> {
        let mut body: HashMap<&String, &String> = HashMap::new();
        for arg in args.body.iter() {
            body.insert(&arg.key, &arg.value);
        }
        let res = client.post(&args.url).json(&body).send().await?;
        Ok(print_response(res).await?)
    }
    async fn put(client: Client, args: &PUT) -> Result<()> {
        let mut body: HashMap<&String, &String> = HashMap::new();
        for arg in args.body.iter() {
            body.insert(&arg.key, &arg.value);
        }
        let res = client.put(&args.url).json(&body).send().await?;
        Ok(print_response(res).await?)
    }
    async fn patch(client: Client, args: &PATCH) -> Result<()> {
        let mut body: HashMap<&String, &String> = HashMap::new();
        for arg in args.body.iter() {
            body.insert(&arg.key, &arg.value);
        }
        let res = client.patch(&args.url).json(&body).send().await?;
        Ok(print_response(res).await?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let mut headers = header::HeaderMap::new();
    headers.insert("X-POWERED-BY", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust HTTPIE".parse()?);
    let client = Client::builder().default_headers(headers).build()?;
    let result = match opts.subcmd {
        SubCommand::Get(ref args) => HttpRequest::get(client, args).await?,
        SubCommand::Post(ref args) => HttpRequest::post(client, args).await?,
        SubCommand::Put(ref args) => HttpRequest::put(client, args).await?,
        SubCommand::Patch(ref args) => HttpRequest::patch(client, args).await?,
    };
    Ok(result)
}

// 仅在 cargo test 时才编译
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_url_works() {
        assert!(parse_url("abc").is_err());
        assert!(parse_url("http://abc.xyz").is_ok());
        assert!(parse_url("https://httpbin.org/post").is_ok());
    }
    #[test]
    fn parse_kv_pair_works() {
        assert!(parse_kv_pair("a").is_err());
        assert_eq!(
            parse_kv_pair("a=1").unwrap(),
            KvPair {
                key: "a".into(),
                value: "1".into()
            }
        );
        assert_eq!(
            parse_kv_pair("b=").unwrap(),
            KvPair {
                key: "b".into(),
                value: "".into()
            }
        );
    }
}
