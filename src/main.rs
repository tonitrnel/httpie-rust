use clap::Parser;
use anyhow::{anyhow, Result};
use url::{Url, ParseError};
use std::str::FromStr;
use reqwest::{header, Client, Response};

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
enum SubCommand {
    Get(GET),
    Post(POST),
}

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Tonitr <tonitrnel@outlook.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Debug)]
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

#[tokio::main]
fn main() {
    let opts: Opts = Opts::parse();
    let client = Client::new();
    let result = match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?,
        SubCommand::Post(ref args) => post(client, args).await?
    };
    Ok(result);
}