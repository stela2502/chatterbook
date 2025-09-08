use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use clap::Parser;

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

#[derive(Debug, Deserialize)]
struct Message {
    author: Author,
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Author {
    role: String,
}

#[derive(Debug, Deserialize)]
struct Content {
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
        }

        fs::write(&filename, md)?;
        println!("Wrote {}", filename.display());
    }

    Ok(())
}