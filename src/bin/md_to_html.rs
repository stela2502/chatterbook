use clap::Parser;
use std::fs;
use std::path::PathBuf;

/// Simple Markdown to HTML converter
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input Markdown file
    #[arg(short, long)]
    input: PathBuf,

    /// Output HTML file
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Read Markdown file
    let md_content = fs::read_to_string(&args.input)?;
    
    // Convert to HTML using pulldown-cmark
    let parser = pulldown_cmark::Parser::new(&md_content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("html");
        path
    });

    fs::write(output_path, html_output)?;
    println!("✅ Markdown converted to HTML successfully.");

    Ok(())
}