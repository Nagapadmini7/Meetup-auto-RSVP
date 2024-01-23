use reqwest::{header, Client, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
struct MeetupEvent {
    id: String,
    name: String,
    time: i64,
}

#[derive(Debug, Deserialize)]
struct MeetupGroup {
    id: String,
    name: String,
}

fn print_success_message(event_name: &str) {
    println!("RSVPing for '{}' - It's going to be awesome!", event_name);
}

fn print_error_message(group_name: &str, error: &str) {
    eprintln!(
        "Oops! Couldn't get ready for {}: {}. Keep the party spirit high! 🚀",
        group_name, error
    );
}

async fn auto_rsvp(api_key: &str, group_name: &str) -> Result<(), reqwest::Error> {
    let meetup_api_url = format!("https://api.meetup.com/{}", group_name);

    let response = Client::new()
        .get(&meetup_api_url)
        .query(&[("key", api_key)])
        .send()
        .await?;

    if response.status().is_success() {
        let group_info: MeetupGroup = response.json().await?;
        let events: Vec<MeetupEvent> = Client::new()
            .get(&format!("{}/events", meetup_api_url))
            .query(&[("status", "upcoming"), ("key", api_key)])
            .send()
            .await?
            .json()
            .await?;

        for event in events {
            let current_time = chrono::Utc::now().timestamp();
            let event_time = event.time / 1000;

            if event_time > current_time {
                print_success_message(&event.name);
                rsvp_event(api_key, &group_info.id, &event.id).await?;
                sleep(Duration::from_secs(5)).await;
            }
        }
    } else if response.status() == StatusCode::TOO_MANY_REQUESTS {
        if let Some(retry_after) = response.headers().get("Retry-After") {
            let retry_seconds = retry_after
                .to_str()
                .unwrap_or("5")
                .parse::<u64>()
                .unwrap_or(5);
            println!(
                "Rate limited. Waiting for {} seconds before retrying...",
                retry_seconds
            );
            sleep(Duration::from_secs(retry_seconds)).await;
        }
    } else {
        print_error_message(group_name, "Unknown error");
    }

    Ok(())
}

async fn rsvp_event(api_key: &str, group_id: &str, event_id: &str) -> Result<(), reqwest::Error> {
    let rsvp_url = format!(
        "https://api.meetup.com/{}/events/{}/rsvps",
        group_id, event_id
    );

    let client = Client::new();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("meetup-auto-rsvp"),
    );

    let params: HashMap<&str, &str> = [("key", api_key), ("response", "yes")]
        .iter()
        .cloned()
        .collect();

    let response = client
        .post(&rsvp_url)
        .headers(headers)
        .json(&params)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Hooray! You're officially in for the event! 🎉");
    } else {
        println!("Uh-oh! Couldn't RSVP for the event. Don't worry, we'll get it next time. 🌈");
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let meetup_api_key = env::var("MEETUP_API_KEY").expect("MEETUP_API_KEY not set");
    let groups_to_rsvp = vec!["group1", "group2"];

    for group in groups_to_rsvp {
        if let Err(err) = auto_rsvp(&meetup_api_key, group).await {
            eprintln!(
                "Oops! Couldn't get ready for {}: {}. Keep the party spirit high! 🚀",
                group, err
            );
        }
    }
}
