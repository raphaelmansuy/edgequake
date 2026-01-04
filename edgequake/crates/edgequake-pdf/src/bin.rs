//! EdgeQuake PDF CLI - Convert PDFs to Markdown with optional LLM vision OCR.
//!
//! # Usage
//!
//! ```bash
//! # Simple conversion (output goes to input.md)
//! edgequake-pdf input.pdf
//!
//! # Explicit convert command
//! edgequake-pdf convert -i input.pdf -o output.md
//!
//! # Enable LLM vision for image OCR
//! edgequake-pdf convert -i input.pdf --vision
//!
//! # Get PDF metadata
//! edgequake-pdf info -i input.pdf
//! ```

use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use edgequake_llm::providers::mock::MockProvider;
use edgequake_pdf::{ExtractionMode, ImageOcrConfig, PdfConfig, PdfExtractor};

/// EdgeQuake PDF - High-quality PDF to Markdown converter
///
/// Converts PDF documents to clean Markdown with advanced layout detection,
/// table extraction, and optional LLM-powered image OCR.
#[derive(Parser)]
#[command(name = "edgequake-pdf")]
#[command(version)]
#[command(author = "EdgeQuake Team")]
#[command(about = "Convert PDFs to Markdown with optional LLM vision OCR")]
#[command(
    long_about = "EdgeQuake PDF is a high-quality PDF to Markdown converter featuring:\n\n\
  • Advanced multi-column layout detection\n\
  • Table extraction with proper Markdown formatting\n\
  • Code block detection with syntax preservation\n\
  • Optional LLM-powered image OCR for figures and charts\n\n\
Examples:\n\
  edgequake-pdf document.pdf                    # Convert to document.md\n\
  edgequake-pdf convert -i doc.pdf -o out.md    # Explicit output path\n\
  edgequake-pdf convert -i doc.pdf --vision     # Enable image OCR\n\
  edgequake-pdf info -i document.pdf            # Show PDF metadata"
)]
struct Cli {
    /// Input PDF file path (shorthand for 'convert -i <FILE>')
    #[arg(value_name = "PDF_FILE")]
    input: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Output markdown file path (defaults to input with .md extension)
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    /// Enable quiet mode (only output errors)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a PDF file to Markdown
    #[command(visible_alias = "c")]
    Convert {
        /// Input PDF file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output markdown file path (defaults to input with .md extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Enable LLM vision mode for image OCR
        ///
        /// When enabled, images and figures in the PDF will be processed
        /// using a vision-capable LLM to extract text and descriptions.
        /// Requires OPENAI_API_KEY environment variable.
        #[arg(long)]
        vision: bool,

        /// LLM model to use for vision OCR (default: gpt-4o-mini)
        #[arg(long, default_value = "gpt-4o-mini")]
        vision_model: String,

        /// Include page numbers in output as comments
        #[arg(long)]
        page_numbers: bool,

        /// Maximum number of pages to process (default: all)
        #[arg(long)]
        max_pages: Option<usize>,

        /// Output format
        #[arg(long, value_enum, default_value = "markdown")]
        format: OutputFormat,

        /// Write output to stdout instead of file
        #[arg(long)]
        stdout: bool,
    },

    /// Display information about a PDF file
    #[command(visible_alias = "i")]
    Info {
        /// Input PDF file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output format for info
        #[arg(long, value_enum, default_value = "text")]
        format: InfoFormat,
    },

    /// Read PDF from stdin and convert to Markdown
    #[command(visible_alias = "p")]
    Pipe {
        /// Enable LLM vision mode for image OCR
        #[arg(long)]
        vision: bool,

        /// Include page numbers in output
        #[arg(long)]
        page_numbers: bool,
    },
}

#[derive(ValueEnum, Clone, Debug, Default)]
enum OutputFormat {
    /// Standard Markdown
    #[default]
    Markdown,
    /// JSON document structure
    Json,
}

#[derive(ValueEnum, Clone, Debug, Default)]
enum InfoFormat {
    /// Human-readable text
    #[default]
    Text,
    /// JSON format
    Json,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else if !cli.quiet {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(false)
            .init();
    }

    // Handle shorthand: edgequake-pdf input.pdf
    if let Some(input) = cli.input {
        return convert_pdf(ConvertOptions {
            input,
            output: cli.output,
            vision: false,
            vision_model: "gpt-4o-mini".to_string(),
            page_numbers: false,
            max_pages: None,
            format: OutputFormat::Markdown,
            stdout: false,
            quiet: cli.quiet,
        })
        .await;
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::Convert {
            input,
            output,
            vision,
            vision_model,
            page_numbers,
            max_pages,
            format,
            stdout,
        }) => {
            convert_pdf(ConvertOptions {
                input,
                output: output.or(cli.output),
                vision,
                vision_model,
                page_numbers,
                max_pages,
                format,
                stdout,
                quiet: cli.quiet,
            })
            .await?;
        }
        Some(Commands::Info { input, format }) => {
            show_pdf_info(input, format, cli.quiet).await?;
        }
        Some(Commands::Pipe {
            vision,
            page_numbers,
        }) => {
            pipe_convert(vision, page_numbers).await?;
        }
        None => {
            // No input and no command - show help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}

/// Options for PDF conversion
struct ConvertOptions {
    input: PathBuf,
    output: Option<PathBuf>,
    vision: bool,
    vision_model: String,
    page_numbers: bool,
    max_pages: Option<usize>,
    format: OutputFormat,
    stdout: bool,
    quiet: bool,
}

async fn convert_pdf(opts: ConvertOptions) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input exists
    if !opts.input.exists() {
        return Err(format!("Input file not found: {}", opts.input.display()).into());
    }

    if !opts
        .input
        .extension()
        .map_or(false, |e| e.eq_ignore_ascii_case("pdf"))
    {
        eprintln!("⚠️  Warning: Input file does not have .pdf extension");
    }

    // Create provider (use OpenAI if vision enabled and API key available)
    let provider: Arc<dyn edgequake_llm::traits::LLMProvider> = if opts.vision {
        match std::env::var("OPENAI_API_KEY") {
            Ok(api_key) => Arc::new(
                edgequake_llm::providers::openai::OpenAIProvider::new(api_key)
                    .with_model(&opts.vision_model),
            ),
            Err(_) => {
                eprintln!("⚠️  Warning: --vision requires OPENAI_API_KEY environment variable");
                eprintln!("   Falling back to non-vision mode");
                Arc::new(MockProvider::new())
            }
        }
    } else {
        Arc::new(MockProvider::new())
    };

    // Build configuration
    let mut config = PdfConfig::new()
        .with_mode(if opts.vision {
            ExtractionMode::Vision
        } else {
            ExtractionMode::Text
        })
        .with_page_numbers(opts.page_numbers);

    if let Some(max_pages) = opts.max_pages {
        config = config.with_max_pages(max_pages);
    }

    // Enable image OCR if vision mode
    if opts.vision {
        config = config.with_image_ocr(ImageOcrConfig {
            enabled: true,
            model: opts.vision_model.clone(),
            ..Default::default()
        });
    }

    let extractor = PdfExtractor::with_config(provider, config);

    // Read PDF
    let pdf_bytes = std::fs::read(&opts.input)?;

    // Extract content
    let output_content = match opts.format {
        OutputFormat::Markdown => extractor.extract_to_markdown(&pdf_bytes).await?,
        OutputFormat::Json => {
            let doc = extractor.extract_document(&pdf_bytes).await?;
            serde_json::to_string_pretty(&doc)?
        }
    };

    // Write output
    if opts.stdout {
        print!("{}", output_content);
    } else {
        let output_path = opts.output.unwrap_or_else(|| {
            let mut path = opts.input.clone();
            path.set_extension("md");
            path
        });

        std::fs::write(&output_path, &output_content)?;

        if !opts.quiet {
            let format_name = match opts.format {
                OutputFormat::Markdown => "Markdown",
                OutputFormat::Json => "JSON",
            };
            println!(
                "✅ Converted {} to {}",
                opts.input.display(),
                output_path.display()
            );
            println!("📄 {} ({} bytes)", format_name, output_content.len());
            if opts.vision {
                println!("🔍 Vision OCR: enabled (model: {})", opts.vision_model);
            }
        }
    }

    Ok(())
}

async fn show_pdf_info(
    input: PathBuf,
    format: InfoFormat,
    quiet: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !input.exists() {
        return Err(format!("Input file not found: {}", input.display()).into());
    }

    let provider = Arc::new(MockProvider::new());
    let extractor = PdfExtractor::new(provider);

    let pdf_bytes = std::fs::read(&input)?;
    let info = extractor.get_info(&pdf_bytes)?;

    match format {
        InfoFormat::Text => {
            if !quiet {
                println!("📋 PDF Information");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }
            println!("  File:       {}", input.display());
            println!("  Pages:      {}", info.page_count);
            println!("  Version:    {}", info.pdf_version);
            println!(
                "  Size:       {} bytes ({:.2} KB)",
                info.file_size,
                info.file_size as f64 / 1024.0
            );
            println!(
                "  Has images: {}",
                if info.has_images { "yes" } else { "no" }
            );
            if info.has_images {
                println!("  Images:     {}", info.image_count);
            }
        }
        InfoFormat::Json => {
            let json = serde_json::json!({
                "file": input.display().to_string(),
                "pages": info.page_count,
                "version": info.pdf_version,
                "size_bytes": info.file_size,
                "has_images": info.has_images,
                "image_count": info.image_count,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

async fn pipe_convert(vision: bool, page_numbers: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Read PDF from stdin
    let mut pdf_bytes = Vec::new();
    io::stdin().read_to_end(&mut pdf_bytes)?;

    if pdf_bytes.is_empty() {
        return Err("No input received from stdin".into());
    }

    // Create provider
    let provider: Arc<dyn edgequake_llm::traits::LLMProvider> =
        match std::env::var("OPENAI_API_KEY") {
            Ok(api_key) if vision => Arc::new(
                edgequake_llm::providers::openai::OpenAIProvider::new(api_key),
            ),
            _ => Arc::new(MockProvider::new()),
        };

    let mut config = PdfConfig::new()
        .with_mode(if vision {
            ExtractionMode::Vision
        } else {
            ExtractionMode::Text
        })
        .with_page_numbers(page_numbers);

    if vision {
        config = config.with_image_ocr_enabled();
    }

    let extractor = PdfExtractor::with_config(provider, config);
    let markdown = extractor.extract_to_markdown(&pdf_bytes).await?;

    io::stdout().write_all(markdown.as_bytes())?;

    Ok(())
}
