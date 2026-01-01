use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{PdfExtractor, PdfConfig, ExtractionMode};

#[derive(Parser)]
#[command(name = "edgequake-pdf")]
#[command(about = "PDF to Markdown conversion tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a PDF file to markdown
    Convert {
        /// Input PDF file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output markdown file path (optional, defaults to input with .md extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Use vision mode for complex documents
        #[arg(long)]
        vision: bool,

        /// Include page numbers in output
        #[arg(long)]
        page_numbers: bool,

        /// Maximum number of pages to process
        #[arg(long)]
        max_pages: Option<usize>,
    },
    /// Get information about a PDF file
    Info {
        /// Input PDF file path
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            vision,
            page_numbers,
            max_pages,
        } => {
            convert_pdf(input, output, vision, page_numbers, max_pages).await?;
        }
        Commands::Info { input } => {
            show_pdf_info(input).await?;
        }
    }

    Ok(())
}

async fn convert_pdf(
    input: PathBuf,
    output: Option<PathBuf>,
    vision: bool,
    page_numbers: bool,
    max_pages: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create extractor
    let provider = Arc::new(MockProvider::new());
    let config = PdfConfig::new()
        .with_mode(if vision { ExtractionMode::Vision } else { ExtractionMode::Text })
        .with_page_numbers(page_numbers);

    let config = if let Some(max_pages) = max_pages {
        config.with_max_pages(max_pages)
    } else {
        config
    };

    let extractor = PdfExtractor::with_config(provider, config);

    // Read PDF
    let pdf_bytes = std::fs::read(&input)?;

    // Extract markdown
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("md");
        path
    });

    // Write output
    std::fs::write(&output_path, &markdown)?;

    println!("✅ Converted {} to {}", input.display(), output_path.display());
    println!("📄 {} characters extracted", markdown.len());

    Ok(())
}

async fn show_pdf_info(input: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let pdf_bytes = std::fs::read(&input)?;
    let info = extractor.get_info(&pdf_bytes)?;

    println!("📋 PDF Information:");
    println!("  File: {}", input.display());
    println!("  Pages: {}", info.page_count);
    println!("  Version: {}", info.pdf_version);
    println!("  Size: {} bytes", info.file_size);
    println!("  Has images: {}", info.has_images);
    if info.has_images {
        println!("  Image count: {}", info.image_count);
    }

    Ok(())
}