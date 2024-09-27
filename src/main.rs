use clap::{Arg, Command};
use reqwest::blocking::Client;
use reqwest::Error as ReqwestError;
use serde_json::Value;
use std::fs;
use std::process::exit;
use std::collections::HashMap;
use regex::Regex;
use html_escape::decode_html_entities;
use select::document::Document;
use select::predicate::Name;
use chrono::{DateTime, Utc};

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
    // Command line options
    let matches = Command::new("mastodon_user_info")
        .version("1.0")
        .about("Get all your Mastodon Posts as Diary")
        .arg(Arg::new("url")
            .help("URL of the Mastodon profile")
            .default_value("https://mastodon.example/@username")
        )
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_parser(clap::value_parser!(String))
            .default_value("posts.html")
        )
        .arg(Arg::new("template")
            .short('t')
            .long("template")
            .value_parser(clap::value_parser!(String))
            .default_value("templates/retro_light.html")
        )
        .get_matches();
    
    let mastodon_url = matches.get_one::<String>("url").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let template_file = matches.get_one::<String>("template").unwrap();

    // Parse Mastodon URL
    let mastodon_host = match reqwest::Url::parse(mastodon_url) {
        Ok(parsed_url) => parsed_url.host_str().unwrap_or("").to_string(),
        Err(_) => {
            eprintln!("Invalid Mastodon URL");
            exit(1);
        }
    };

    let mastodon_username = mastodon_url.split('/').last().unwrap();

    let user_lookup_api = format!("https://{}/api/v1/accounts/lookup?acct={}", mastodon_host, mastodon_username);

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

    let user_timeline_api = format!("https://{}/api/v1/accounts/{}/statuses", mastodon_host, user_id);
    
    let mut all_posts = Vec::new();
    let mut params = HashMap::new();
    
    // Fetch all posts
    loop {
        let response: Vec<Value> = client.get(&user_timeline_api)
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
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
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
