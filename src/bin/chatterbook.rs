use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::path::Path;

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
#[allow(dead_code)]
struct MessageEntry {
    id: String,
    message: Option<Message>,        // the nested "message" object
    parent: Option<String>,
    children: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct Message {
    id: String,
    author: Author,
    create_time: Option<f64>,
    update_time: Option<f64>,
    content: Content,
    status: String,
    end_turn: Option<bool>,
    weight: f64,
    metadata: serde_json::Value,
    recipient: Option<String>,
    channel: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Part {
    Text(String),
    Image {
        content_type: String,
        asset_pointer: Option<String>,
        size_bytes: Option<u64>,
        width: Option<u32>,
        height: Option<u32>,
        fovea: Option<serde_json::Value>,
        metadata: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct Author {
    role: String,
    name: Option<String>,
    metadata: serde_json::Value,
}



#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Thought {
    summary: String,
    content: String,
    #[serde(default)]
    chunks: Vec<String>,
    finished: bool,
}


#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ContentReference {
    matched_text: String,
    safe_urls: Vec<String>,
    // add more fields as needed
}


#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "content_type")]
#[allow(dead_code)]
enum Content {
    #[serde(rename = "text")]
    Text { parts: Vec<Part> },

    #[serde(rename = "thoughts")]
    Thoughts {
        thoughts: Vec<Thought>,
        #[serde(default)]
        source_analysis_msg_id: Option<String>,
    },
    #[serde(rename = "code")]
    Code {
        language: Option<String>,
        text: String,
        response_format_name: Option<String>,
    },
    #[serde(rename = "reasoning_recap")]
    ReasoningRecap {
        content: String,
        #[serde(default)]
        content_references: Option<Vec<ContentReference>>,
    },
    #[serde(rename = "multimodal_text")]
    MultimodalText {
        parts: Vec<MultimodalPart>,
    },
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "content_type")]
#[allow(dead_code)]
enum MultimodalPart {
    #[serde(rename = "image_asset_pointer")]
    ImageAssetPointer {
        asset_pointer: String,
        size_bytes: u64,
        width: u32,
        height: u32,
        fovea: Option<serde_json::Value>,
        metadata: ImageMetadata,
    },
    // you can add more part types here if needed
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct ImageMetadata {
    dalle: Option<DalleMetadata>,
    gizmo: Option<serde_json::Value>,
    generation: Option<GenerationMetadata>,
    container_pixel_height: Option<u32>,
    container_pixel_width: Option<u32>,
    emu_omit_glimpse_image: Option<serde_json::Value>,
    emu_patches_override: Option<serde_json::Value>,
    lpe_keep_patch_ijhw: Option<serde_json::Value>,
    sanitized: Option<bool>,
    asset_pointer_link: Option<String>,
    watermarked_asset_pointer: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct DalleMetadata {
    gen_id: String,
    prompt: String,
    seed: Option<u64>,
    parent_gen_id: Option<String>,
    edit_op: Option<String>,
    serialization_title: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct GenerationMetadata {
    gen_id: String,
    gen_size: String,
    seed: Option<u64>,
    parent_gen_id: Option<String>,
    height: u32,
    width: u32,
    transparent_background: bool,
    serialization_title: Option<String>,
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Search for a PNG matching `file_id` in any `user-*` folder inside `base_folder`.
fn find_png(file_id: &str, base_folder: &Path) -> Option<PathBuf> {
    // Iterate over entries in the base folder
    for entry in fs::read_dir(base_folder).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        // Only consider directories whose name starts with "user-"
        if path.is_dir() {
            if let Some(folder_name) = path.file_name().and_then(|f| f.to_str()) {
                if folder_name.starts_with("user-") {
                    // Look for the PNG inside this folder
                    for file_entry in fs::read_dir(&path).ok()? {
                        let file_entry = file_entry.ok()?;
                        let file_path = file_entry.path();
                        if let Some(fname) = file_path.file_name().and_then(|f| f.to_str()) {
                            if fname.starts_with(file_id) && fname.ends_with(".png") {
                                return Some(file_path);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn find_png_for_asset(asset_pointer: &str, user_folder: &Path, out_folder: &Path, figure_base: &Path,) -> Option<String> {
    // Extract the file ID after "sediment://"
    let file_id = asset_pointer.strip_prefix("sediment://")?;

    if let Some(png_src) = find_png( file_id, user_folder ){
        // Create new filename matching the Markdown file
        let asset_stem = Path::new(&png_src).file_stem()?.to_string_lossy();
        let new_png_path = out_folder.join(format!(
            "{}_{}.png",
            figure_base.display(),
            asset_stem
        ));
        // Copy the PNG
        println!("I am copying {} to {} - right?", &png_src.display(), &new_png_path.display());
        if let Err(e) = fs::copy(Path::new(&png_src), &new_png_path) {
            eprintln!(
                "Failed to copy {} to {}: {}",
                png_src.display(),
                new_png_path.display(),
                e
            );
            return None;
        }
        // Return just the filename for Markdown
        Some(new_png_path.file_name()?.to_string_lossy().to_string())
    }else {
        None
    }
}




fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Read JSON
    let data = fs::read_to_string(&args.infile)?;
    let parent_folder = args.infile.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let conversations: Vec<Conversation> = serde_json::from_str(&data)?;

    fs::create_dir_all(&args.outpath)?;

    for conv in conversations {
        let title = conv.title.clone().unwrap_or_else(|| "untitled".to_string());
        if title == "New chat"{
            continue;
        }
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
        let figure_base = args.outpath.join(format!("conversation_{}_{}", save_time,safe_title));
        let mut sections = 0;

        if let Some(mapping) = conv.mapping.as_object() {

            
            let messages_map: HashMap<String, MessageEntry> = mapping
                .values()
                .filter_map(|v| serde_json::from_value::<MessageEntry>(v.clone()).ok())
                .map(|entry| {
                    //let first_child = entry.children.first().cloned();
                    (entry.id.clone(), entry)
                })
                .collect();

            let root_id = messages_map
                .values()
                .find(| entry | entry.parent.is_none())
                .map(| entry | entry.children.first().unwrap().clone())
                .unwrap_or_default();

            #[cfg(debug_assertions)]
            println!("I got the root id '{}'", root_id );
            // Only traverse if root exists
            if !root_id.is_empty() {
                let mut id = &root_id;  
                while let Some(entry) = messages_map.get(id) {
                    if let Some(msg) = &entry.message {
                        let role = match msg.author.role.as_str() {
                            "user" => "👤 User",
                            "assistant" => "🤖 Assistant",
                            other => other,
                        };

                        let content = match &msg.content {
                            Content::Text { parts } => parts
                                .iter()
                                .filter_map(|p| match p { Part::Text(s) => Some(s.as_str()), &Part::Image { .. } => todo!() })
                                .collect::<Vec<_>>()
                                .join("\n"),

                            Content::MultimodalText { parts } => parts
                                .iter()
                                .map(|p| 
                                    match p { MultimodalPart::ImageAssetPointer { asset_pointer, .. } => {
                                        // Convert asset_pointer to a PNG path
                                        // Assuming you have a function like `find_png_for_asset`
                                        match find_png_for_asset(asset_pointer, &parent_folder, &args.outpath, &figure_base ) {
                                            Some(png_path) => format!("![]({})", png_path),
                                            None => format!("![Missing image for {}]", asset_pointer),
                                        }
                                    }
                                    // handle other MultimodalPart types here if you have them
                                })
                                .collect::<Vec<_>>()
                                .join("\n\n"),

                            Content::Thoughts { thoughts, .. } => thoughts
                                .iter()
                                .map(|t| format!("**{}**\n{}", t.summary, t.content))
                                .collect::<Vec<_>>()
                                .join("\n\n"),
                            Content::Code { language, text, .. } => {
                                let lang = language.as_deref().unwrap_or("text");
                                format!("```{}\n{}\n```", lang, text)
                            },
                            Content::ReasoningRecap { content, content_references } => {
                                let mut md = content.clone();
                                if let Some(refs) = content_references {
                                    for r in refs {
                                        if !r.safe_urls.is_empty() {
                                            md.push_str(&format!("\n[Reference]({})", r.safe_urls[0]));
                                        }
                                    }
                                }
                                md
                            },
                        };

                        sections += 1;
                        md.push_str(&format!("**{}:**\n\n{}\n\n---\n\n", role, content));
                    }

                    //if let Some(first_child) = entry.children.first() {
                    if let Some(first_child) = entry.children.last() {
                        #[cfg(debug_assertions)]
                        println!("Got the message id {id} -> with msg {:?}", first_child);
                        id = first_child;
                    } else {
                        break;
                    }
                }
                #[cfg(debug_assertions)]
                if sections == 0 {
                    println!("But I could not identify the entry!");
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
        if sections > 0 {
            fs::write(&filename, md)?;
            println!("Wrote {}", filename.display());
        }else {
            println!("ERROR: Failed to detect content for '{}' - file {}", title, filename.display() );
        }
        
        
        
    }

    Ok(())
}




