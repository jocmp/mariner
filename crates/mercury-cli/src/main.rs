//! `mercury-parser`: extract the semantic content from a URL.

use std::process::ExitCode;

use clap::{Parser as _, ValueEnum};
use mercury_parser::{ContentType, CustomExtractor, Error, Field, ParseOptions, Parser};

#[derive(clap::Parser)]
#[command(
    version,
    about = "Mercury Parser extracts semantic content from any url"
)]
struct Args {
    /// The URL to parse.
    url: String,

    /// The format of the extracted content.
    #[arg(short, long, value_enum, default_value_t = Format::Html)]
    format: Format,

    /// A request header, as `name=value`. May be repeated.
    #[arg(short = 'H', long = "header", value_name = "NAME=VALUE")]
    headers: Vec<String>,

    /// Extract an extra field, as `name=selector` or `name=selector|attribute`.
    #[arg(short, long, value_name = "NAME=SELECTOR")]
    extend: Vec<String>,

    /// Like --extend, but collect every match into a list.
    #[arg(short = 'l', long, value_name = "NAME=SELECTOR")]
    extend_list: Vec<String>,

    /// Parse this local HTML file instead of fetching the URL.
    #[arg(long, value_name = "PATH")]
    html: Option<std::path::PathBuf>,

    /// Don't follow next-page links.
    #[arg(long)]
    no_all_pages: bool,

    /// A JSON extractor definition to use for this URL.
    #[arg(short = 'x', long, value_name = "PATH")]
    add_extractor: Option<std::path::PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Html,
    #[value(alias = "md")]
    Markdown,
    #[value(alias = "txt")]
    Text,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();

    let mut options = ParseOptions::new()
        .fetch_all_pages(!args.no_all_pages)
        .content_type(match args.format {
            Format::Html => ContentType::Html,
            Format::Markdown => ContentType::Markdown,
            Format::Text => ContentType::Text,
        });

    if let Some(path) = &args.html {
        match std::fs::read_to_string(path) {
            Ok(html) => options = options.html(html),
            Err(error) => {
                eprintln!("Unable to read {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    for header in &args.headers {
        let Some((name, value)) = header.split_once('=') else {
            eprintln!("Headers must look like name=value: {header}");
            return ExitCode::FAILURE;
        };
        options = options.header(name, value);
    }

    for (spec, allow_multiple) in args
        .extend
        .iter()
        .map(|spec| (spec, false))
        .chain(args.extend_list.iter().map(|spec| (spec, true)))
    {
        let Some((name, selector)) = spec.split_once('=') else {
            eprintln!("Extensions must look like name=selector: {spec}");
            return ExitCode::FAILURE;
        };
        let field = match selector.split_once('|') {
            Some((css, attr)) => Field::new().attr(css, attr),
            None => Field::new().selector(selector),
        };
        options = options.extend(name, field.allow_multiple(allow_multiple));
    }

    if let Some(path) = &args.add_extractor {
        let extractor = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|json| CustomExtractor::from_json(&json).map_err(|e| e.to_string()));
        match extractor {
            Ok(extractor) => options = options.custom_extractor(extractor),
            Err(error) => {
                eprintln!("Unable to load extractor from {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    match Parser::new().parse(&args.url, options).await {
        Ok(result) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("results serialize")
            );
            ExitCode::SUCCESS
        }
        // Upstream reports these as a result object rather than a crash.
        Err(error @ (Error::InvalidUrl | Error::Fetch(_))) => {
            let result = serde_json::json!({ "error": true, "message": error.to_string() });
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("results serialize")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("\nMercury Parser encountered a problem trying to parse that resource.\n");
            eprintln!("{error}");
            eprintln!(
                "\nIf you believe this was an error, please file an issue at:\n\n    https://github.com/jocmp/mercury-parser-rs/issues/new\n"
            );
            ExitCode::FAILURE
        }
    }
}
