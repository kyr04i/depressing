use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use chrono::{NaiveDateTime, Duration};
use tokio::sync::Mutex; 
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration as StdDuration; 

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Depressing v2.0 by Yi Nguyen")]
enum Command {
    #[command(description = "Display this text")]
    Help,
    
    #[command(description = "Set a deadline: /set <name> <date> <time> <duration> <frequency>")]
    Set,
    
    #[command(description = "View all deadlines")]
    View,
    
    #[command(description = "Delete a deadline: /delete <name>")]
    Delete,

    #[command(description = "Get your Chat ID")]
    MyId,
}

struct Deadline {
    name: String,
    datetime: NaiveDateTime,
    duration: Duration,
    frequency: String,
    chat_ids: Vec<ChatId>, 
}

type DeadlineStorage = Arc<Mutex<HashMap<String, Deadline>>>; 

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting deadline bot...");

    let bot = Bot::from_env();
    let deadlines = DeadlineStorage::default();

    let deadlines_clone = Arc::clone(&deadlines);
    let bot_clone = bot.clone(); 
    tokio::spawn(async move {
        loop {
            let now = chrono::Local::now().naive_local();
            let mut reminders = Vec::new();
            {
                let deadlines = deadlines_clone.lock().await; 
                for (name, deadline) in deadlines.iter() {
                    // Check if the deadline is due
                    if now >= deadline.datetime && now <= deadline.datetime + deadline.duration {
                        reminders.push((name.clone(), deadline.chat_ids.clone()));
                    }
                }
            }

            for (name, chat_ids) in reminders {
                for chat_id in chat_ids {
                    bot_clone.send_message(chat_id, format!("Reminder: Deadline '{}' is due!", name)).await.unwrap();
                }
            }

            tokio::time::sleep(StdDuration::from_secs(60)).await;
        }
    });

    Command::repl(bot, move |bot: Bot, msg: Message, cmd: Command| {
        let deadlines = Arc::clone(&deadlines);
        async move {
            match cmd {
                Command::Help => {
                    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
                }
                Command::Set => {
                    let text = msg.text().unwrap_or_default();
                    let parts: Vec<&str> = text.split_whitespace().collect();

                    if parts.len() != 6 { 
                        bot.send_message(msg.chat.id, "Invalid format. Use: /set <name> <date> <time> <duration> <frequency>").await?;
                        return Ok(());
                    }

                    let name = parts[1].to_string();
                    let date = parts[2].to_string();
                    let time = parts[3].to_string();
                    let duration = parts[4].to_string();
                    let frequency = parts[5].to_string();

                    let datetime = match NaiveDateTime::parse_from_str(&format!("{} {}", date, time), "%Y-%m-%d %H:%M") {
                        Ok(dt) => dt,
                        Err(_) => {
                            bot.send_message(msg.chat.id, "Invalid date or time format. Use YYYY-MM-DD HH:MM.").await?;
                            return Ok(());
                        }
                    };

                    let duration = match duration.parse::<i64>() {
                        Ok(d) => Duration::minutes(d),
                        Err(_) => {
                            bot.send_message(msg.chat.id, "Invalid duration. Please provide a number of minutes.").await?;
                            return Ok(());
                        }
                    };

                    let deadline = Deadline {
                        name: name.clone(),
                        datetime,
                        duration,
                        frequency,
                        chat_ids: vec![msg.chat.id], 
                    };

                    deadlines.lock().await.insert(name.clone(), deadline);
                    bot.send_message(msg.chat.id, format!("Deadline '{}' set successfully!", name)).await?;
                }
                Command::View => {
                    let deadlines = deadlines.lock().await;
                    let mut response = String::new();
                    for (name, deadline) in deadlines.iter() {
                        response.push_str(&format!(
                            "Name: {}\nDate & Time: {}\nDuration: {} minutes\nFrequency: {}\nChat IDs: {:?}\n\n",
                            name,
                            deadline.datetime,
                            deadline.duration.num_minutes(),
                            deadline.frequency,
                            deadline.chat_ids
                        ));
                    }
                    if response.is_empty() {
                        response = "No deadlines set.".to_string();
                    }
                    bot.send_message(msg.chat.id, response).await?;
                }
                Command::Delete => {
                    let text = msg.text().unwrap_or_default(); 
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    
                    if parts.len() != 2 {
                        bot.send_message(msg.chat.id, "Invalid format. Use: /delete <name>").await?;
                        return Ok(());
                    }

                    let name = parts[1].to_string();
                    let mut deadlines = deadlines.lock().await; 
                    if deadlines.remove(&name).is_some() {
                        bot.send_message(msg.chat.id, format!("Deadline '{}' deleted successfully!", name)).await?;
                    } else {
                        bot.send_message(msg.chat.id, format!("Deadline '{}' not found.", name)).await?;
                    }
                }
                Command::MyId => {
                    let chat_id = msg.chat.id;
                    bot.send_message(chat_id, format!("Your Chat ID is: {}", chat_id)).await?;
                }
            }
            Ok(())
        }
    })
    .await;
}
