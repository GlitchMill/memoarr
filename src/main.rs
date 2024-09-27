use chrono::offset::LocalResult;
use chrono::DateTime;
use chrono::TimeZone; // Import the TimeZone trait
use chrono_tz::Tz;
use clap::Command;
use html_escape::decode_html_entities;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use select::document::Document;
use select::predicate::Name;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::process::exit;

#[derive(Deserialize)]
struct Config {
    mastodon_url: String,
    output_file: String,
    template_file: String,
    timezone: String,
}

fn load_config(filename: &str) -> Result<Config, std::io::Error> {
    let file = std::fs::File::open(filename)?;
    let reader = std::io::BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    Ok(config)
}

fn process_diary_content(content: &str) -> Option<String> {
    let doc = Document::from(content);
    let text: String = doc
        .find(Name("body"))
        .map(|n| n.text())
        .collect::<Vec<_>>()
        .join("");

    if text.starts_with("#Diary") {
        let diary_content = text.trim_start_matches("#Diary").trim().to_string();
        // Remove extra spaces, commas, and exclamation marks
        let re = Regex::new(r"[ ,!]+").unwrap();
        return Some(re.replace_all(&diary_content, " ").trim().to_string());
    }
    None
}

fn main() -> Result<(), ReqwestError> {
    let config = load_config("config.json").unwrap_or_else(|_| {
        eprintln!("Error loading config.json");
        std::process::exit(1);
    });

    // No command line arguments, only help
    let _matches = Command::new("mastodon_user_info")
        .version("1.0")
        .about("Get all your Mastodon Posts as Diary")
        .get_matches();

    // Use config values directly
    let mastodon_url = &config.mastodon_url;
    let output_file = &config.output_file;
    let template_file = &config.template_file;
    let timezone_str = &config.timezone;

    // Parse Mastodon URL
    let mastodon_host = match reqwest::Url::parse(mastodon_url) {
        Ok(parsed_url) => parsed_url.host_str().unwrap_or("").to_string(),
        Err(_) => {
            eprintln!("Invalid Mastodon URL");
            exit(1);
        }
    };

    let mastodon_username = mastodon_url.split('/').last().unwrap();

    let user_lookup_api = format!(
        "https://{}/api/v1/accounts/lookup?acct={}",
        mastodon_host, mastodon_username
    );

    // Fetch user data
    let client = Client::new();
    let user_data: Value = client.get(&user_lookup_api).send()?.json()?;

    let user_id = match user_data.get("id") {
        Some(id) => id.as_str().unwrap_or("").to_string(),
        None => {
            eprintln!("This user doesn't exist.");
            exit(1);
        }
    };

    let user_timeline_api = format!(
        "https://{}/api/v1/accounts/{}/statuses",
        mastodon_host, user_id
    );

    let mut all_posts = Vec::new();
    let mut params = HashMap::new();

    // Fetch all posts
    loop {
        let response: Vec<Value> = client
            .get(&user_timeline_api)
            .query(&params)
            .send()?
            .json()?;

        if response.is_empty() {
            break;
        }

        all_posts.extend(response.clone());

        let oldest_post = &response[response.len() - 1];
        let max_id = oldest_post["id"].as_str().unwrap().parse::<i64>().unwrap() - 1;
        params.insert("max_id", max_id.to_string());
    }

    // Load template file
    let template_content = fs::read_to_string(template_file).unwrap_or_else(|_| {
        eprintln!("Template file not found.");
        exit(1);
    });

    // Prepare posts content
    let mut post_entries = Vec::new();
    let mut last_date = None;

    // Get the timezone
    let timezone: Tz = match timezone_str.parse() {
        Ok(tz) => tz,
        Err(_) => {
            eprintln!("Invalid timezone format: {}", timezone_str);
            exit(1);
        }
    };

    // Get current time in the specified timezone
let current_time = match timezone.from_local_datetime(&chrono::Local::now().naive_utc()) {
    LocalResult::Single(dt) => dt,
    LocalResult::Ambiguous(dt1, dt2) => dt1, // or choose dt2 based on your preference
    LocalResult::None => {
        eprintln!("Unable to get current time in specified timezone.");
        exit(1);
    }
};


    for post in &all_posts {
        let created_at = post["created_at"].as_str().unwrap_or("");
        let content = post["content"].as_str().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }

        let diary_content = process_diary_content(content);
        if diary_content.is_none() {
            continue;
        }

        let created_at_dt = DateTime::parse_from_rfc3339(created_at)
            .map(|dt| dt.with_timezone(&timezone))
            .unwrap_or(current_time);

        let formatted_date = created_at_dt.format("%d/%m/%Y").to_string();
        let formatted_time = created_at_dt.format("%I:%M %p").to_string();

        let decoded_content = decode_html_entities(&diary_content.unwrap()).to_string();

        // Only add date header if date changes
        if last_date != Some(formatted_date.clone()) {
            if last_date.is_some() {
                post_entries.push("<hr>".to_string());
            }
            post_entries.push(format!("<p><strong>{}</strong></p>", formatted_date));
            last_date = Some(formatted_date.clone());
        }

        post_entries.push(format!(
            "<article><p class='post-time'>{}</p><p>{}</p></article>",
            formatted_time, decoded_content
        ));
    }

    if post_entries.is_empty() {
        post_entries.push("<p>No posts with content found.</p>".to_string());
    }

    // Replace {{posts}} with actual content in the template
    let html_output = template_content.replace("{{posts}}", &post_entries.join("\n"));

    // Write to output file
    fs::write(output_file, html_output).unwrap_or_else(|_| {
        eprintln!("Error writing to output file.");
        exit(1);
    });

    println!("Posts written to {}", output_file);
    Ok(())
}