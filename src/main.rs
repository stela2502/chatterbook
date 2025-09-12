use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use clap::Parser;
use std::collections::HashMap;

#[derive(Parser)]
#[command(author, version, about = "Convert ChatGPT all data JSON to Markdown")]
struct Args {
    /// Input JSON file (ChatGPT export)
    #[arg(short, long)]
    infile: PathBuf,

    /// Output directory for Markdown files
    #[arg(short, long, default_value = ".")]
    outpath: PathBuf,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Conversation {
    id: String,
    title: Option<String>,
    create_time: Option<f64>,
    mapping: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct MessageEntry {
    id: String,
    message: Message,        // the nested "message" object
    parent: String,
    children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    id: String,
    author: Author,
    create_time: f64,
    update_time: Option<f64>,
    content: Option<Content>,
    status: String,
    end_turn: Option<bool>,
    weight: f64,
    metadata: serde_json::Value,
    recipient: Option<String>,
    channel: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Author {
    role: String,
    name: Option<String>,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct Content {
    content_type: String,
    parts: Vec<String>,
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}



fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Read JSON
    let data = fs::read_to_string(&args.infile)?;
    let conversations: Vec<Conversation> = serde_json::from_str(&data)?;

    fs::create_dir_all(&args.outpath)?;

    for conv in conversations {
        let title = conv.title.clone().unwrap_or_else(|| "untitled".to_string());
        let safe_title = sanitize_filename(&title);
        

        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", title));

        let time = if let Some(ts) = &conv.create_time {
            use chrono::{ Utc, TimeZone};
            let dt = Utc.timestamp_opt(ts.floor() as i64, (ts.fract()*1e9) as u32).unwrap();
            let time = format!("{}", dt);
            md.push_str(&format!("_Created: {}_\n\n", dt));
            time
        }else {
            "unkown".to_string()
        };
        let save_time = sanitize_filename(&time);
        let filename = args.outpath.join(format!("conversation_{}_{}.md", save_time,safe_title));


        if let Some(mapping) = conv.mapping.as_object() {

            let mut id = "".to_string();
            let messages_map: HashMap<String, (Option<String>, MessageEntry)> = mapping
                .values()
                .filter_map(|v| serde_json::from_value::<MessageEntry>(v.clone()).ok())
                .map(|entry| {
                    // take the first child if any
                    let first_child = entry.children.first().cloned();
                    //println!("id {} and first child {:?} and parent {:?}", entry.id, first_child, entry.parent);
                    if id == "".to_string() {
                        id= entry.id.clone();
                    }
                    (entry.id.clone(), (first_child, entry))
                })
                .collect();
            let root_id = messages_map
                .values()
                .find(|(_, entry)| !messages_map.contains_key(&entry.parent))
                .map(|(_,entry)| entry.id.clone())
                .unwrap_or_default();

            
            // Only traverse if root exists
            if !root_id.is_empty() {
                let mut id = &root_id;
                while let Some((_, entry)) = messages_map.get(id) {
                    // process entry.message
                    let role = match entry.message.author.role.as_str() {
                        "user" => "👤 User",
                        "assistant" => "🤖 Assistant",
                        other => other,
                    };

                    let content = entry.message
                        .content
                        .as_ref()
                        .map(|c| c.parts.join("\n"))
                        .unwrap_or_default();

                    md.push_str(&format!("**{}:**\n\n{}\n\n---\n\n", role, content));

                    // move to first child, stop if none
                    if let Some(first_child) = entry.children.first() {
                        id = first_child;
                    } else {
                        break;
                    }
                }
            } else {
                // root missing, skip this entry
                println!("No root message found — skipping this entry");
            }
        };
        /*
            let mut messages: Vec<(String, String)> = Vec::new();

            for (_id, entry) in mapping {
                if let Some(msg) = entry.get("message") {
                    if let Ok(parsed) = serde_json::from_value::<Message>(msg.clone()) {
                        let role = match parsed.author.role.as_str() {
                            "user" => "👤 User",
                            "assistant" => "🤖 Assistant",
                            other => other,
                        };
                        let content = parsed
                            .content
                            .map(|c| c.parts.join("\n"))
                            .unwrap_or_default();
                        messages.push((role.to_string(), content));
                    }
                }
            }

            for (role, content) in messages {
                md.push_str(&format!("**{}:**\n\n{}\n\n---\n\n", role, content));
            }
        }*/

        fs::write(&filename, md)?;
        println!("Wrote {}", filename.display());
    }

    Ok(())
}