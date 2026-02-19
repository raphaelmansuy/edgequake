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

        /// LLM model to use for vision OCR (default: gpt-4.1-nano)
        #[arg(long, default_value = "gpt-4.1-nano")]
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

        /// Extract images from the PDF and save to ./assets/ subfolder (OODA-35)
        ///
        /// When enabled, images embedded in the PDF are extracted as PNG files
        /// and saved to an `assets/` directory next to the output file.
        /// Markdown image references are inserted at page boundaries.
        #[arg(long)]
        extract_images: bool,
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

    // OODA-09: Initialize logging to stderr (not stdout)
    // WHY: CLI tools should output data to stdout and logs to stderr.
    // This allows piping output: `edgequake-pdf input.pdf | head` works correctly.
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .init();
    } else if !cli.quiet {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(false)
            .with_writer(std::io::stderr)
            .init();
    }

    // Handle shorthand: edgequake-pdf input.pdf
    if let Some(input) = cli.input {
        return convert_pdf(ConvertOptions {
            input,
            output: cli.output,
            vision: false,
            vision_model: "gpt-4.1-nano".to_string(),
            page_numbers: false,
            max_pages: None,
            format: OutputFormat::Markdown,
            stdout: false,
            quiet: cli.quiet,
            extract_images: false,
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
            extract_images,
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
                extract_images,
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
    extract_images: bool,
}

async fn convert_pdf(opts: ConvertOptions) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input exists
    if !opts.input.exists() {
        return Err(format!("Input file not found: {}", opts.input.display()).into());
    }

    if !opts
        .input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
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
    let mut output_content = match opts.format {
        OutputFormat::Markdown => extractor.extract_to_markdown(&pdf_bytes).await?,
        OutputFormat::Json => {
            let doc = extractor.extract_document(&pdf_bytes).await?;
            serde_json::to_string_pretty(&doc)?
        }
    };

    // OODA-35: Extract images and save to ./assets/ subfolder
    // WHY: Spec requires "If image is discovered in the PDF they should be
    // extracted in ./assets/ subfolder and linked as image in the transformed
    // markdown as a Markdown image"
    #[cfg(feature = "pdfium")]
    if opts.extract_images && matches!(opts.format, OutputFormat::Markdown) {
        let image_refs = extract_and_save_images(&pdf_bytes, &opts)?;
        if !image_refs.is_empty() {
            output_content = insert_image_references(&output_content, &image_refs);
            if !opts.quiet {
                eprintln!("🖼️  Extracted {} images to assets/", image_refs.len());
            }
        }
    }

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
            if opts.extract_images {
                println!("🖼️  Image extraction: enabled (saved to assets/)");
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

/// Image reference for insertion into markdown (OODA-35).
#[cfg(feature = "pdfium")]
struct ImageRef {
    /// Page number (0-indexed)
    page_num: usize,
    /// Relative path to the saved image file (e.g., "./assets/page0_img0.png")
    path: String,
    /// Alt text for the markdown image
    alt: String,
}

/// Extract images from PDF and save them to ./assets/ subfolder (OODA-35).
///
/// ## Algorithm
///
/// 1. Create PdfiumExtractor to access raw PDF page objects
/// 2. Extract all images with their page positions
/// 3. Create ./assets/ directory relative to the output file
/// 4. Save each image as PNG with unique name: `page{N}_img{M}.png`
/// 5. Return image references for markdown insertion
///
/// ## WHY Separate from Main Pipeline?
///
/// The main pipeline (PdfiumBackend → ProcessorChain → MarkdownRenderer)
/// handles text extraction. Image extraction is a parallel concern that
/// operates on PDF page objects (not text). Keeping them separate:
/// - Avoids bloating the Document struct with binary image data
/// - Allows image extraction to be optional (--extract-images flag)
/// - Keeps the text pipeline clean and focused
#[cfg(feature = "pdfium")]
fn extract_and_save_images(
    pdf_bytes: &[u8],
    opts: &ConvertOptions,
) -> Result<Vec<ImageRef>, Box<dyn std::error::Error>> {
    use edgequake_pdf::backend::pdfium::PdfiumExtractor;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create a fresh PdfiumExtractor for image extraction
    let pdfium_extractor = PdfiumExtractor::new()?;
    let extracted = pdfium_extractor.extract_images_from_bytes(pdf_bytes)?;

    if extracted.is_empty() {
        return Ok(Vec::new());
    }

    // Determine assets directory: ./assets/ relative to the output file
    let output_path = opts.output.clone().unwrap_or_else(|| {
        let mut path = opts.input.clone();
        path.set_extension("md");
        path
    });
    let assets_dir = output_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("assets");

    // Create assets directory
    std::fs::create_dir_all(&assets_dir)?;

    let mut refs = Vec::new();

    for img_data in &extracted {
        // OODA-IT36: Use content hash for idempotent image naming.
        // WHY (spec requirement): "Ensure image name is idempotent (e.g., hash
        // of content) to avoid duplicates and enable caching."
        // Hash the raw pixel data so identical images get the same filename
        // regardless of page position or extraction order.
        let mut hasher = DefaultHasher::new();
        img_data.image.as_bytes().hash(&mut hasher);
        let hash = hasher.finish();
        let filename = format!("img_{:016x}.png", hash);
        let file_path = assets_dir.join(&filename);

        // Skip if file already exists (idempotent: same content = same file)
        if file_path.exists() {
            refs.push(ImageRef {
                page_num: img_data.page_num,
                path: format!("./assets/{}", filename),
                alt: format!(
                    "Image from page {} ({}x{})",
                    img_data.page_num + 1,
                    img_data.width,
                    img_data.height
                ),
            });
            continue;
        }

        // Save image as PNG
        // WHY PNG: Lossless format preserves quality of diagrams, charts, and text.
        // JPEG would lose quality on sharp edges common in PDF figures.
        match img_data
            .image
            .save_with_format(&file_path, image::ImageFormat::Png)
        {
            Ok(()) => {
                refs.push(ImageRef {
                    page_num: img_data.page_num,
                    path: format!("./assets/{}", filename),
                    alt: format!(
                        "Image from page {} ({}x{})",
                        img_data.page_num + 1,
                        img_data.width,
                        img_data.height
                    ),
                });
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Failed to save image page{}_{}: {}",
                    img_data.page_num, img_data.index, e
                );
            }
        }
    }

    Ok(refs)
}

/// Insert image references into markdown at page boundaries (OODA-35).
///
/// ## Algorithm
///
/// Scans the markdown for page boundary markers (`<!-- Page N -->`)
/// and inserts image references at the end of each page's content.
///
/// If no page markers are found, appends all images at the end of the document.
///
/// ## WHY Page Boundary Insertion?
///
/// Images belong to specific pages. Inserting them at page boundaries
/// maintains reading order: all text from a page appears first, then
/// the images from that page, before moving to the next page.
#[cfg(feature = "pdfium")]
fn insert_image_references(markdown: &str, refs: &[ImageRef]) -> String {
    if refs.is_empty() {
        return markdown.to_string();
    }

    // Group images by page
    let mut images_by_page: std::collections::HashMap<usize, Vec<&ImageRef>> =
        std::collections::HashMap::new();
    for r in refs {
        images_by_page.entry(r.page_num).or_default().push(r);
    }

    // Try to find page markers (<!-- Page N -->) and insert images after them
    let mut result = String::with_capacity(markdown.len() + refs.len() * 80);
    let mut inserted_pages = std::collections::HashSet::new();

    for line in markdown.lines() {
        result.push_str(line);
        result.push('\n');

        // Check if this line is a page marker: <!-- Page N -->
        if let Some(page_num) = parse_page_marker(line) {
            // Insert images for the PREVIOUS page (page_num - 1, 0-indexed = page_num - 2)
            // Actually, page markers appear at the START of a page section.
            // So we should insert images for the page that's about to end.
            // But since we're scanning forward, we insert images for the
            // current page at the NEXT page marker (or end of document).
            let prev_page = if page_num >= 2 {
                page_num - 2
            } else {
                continue;
            };
            if let Some(page_images) = images_by_page.get(&prev_page) {
                if !inserted_pages.contains(&prev_page) {
                    result.push('\n');
                    for img_ref in page_images {
                        result.push_str(&format!("![{}]({})\n\n", img_ref.alt, img_ref.path));
                    }
                    inserted_pages.insert(prev_page);
                }
            }
        }
    }

    // Insert any remaining images that weren't placed (last page or no page markers)
    for (page_num, page_images) in &images_by_page {
        if !inserted_pages.contains(page_num) {
            result.push('\n');
            for img_ref in page_images {
                result.push_str(&format!("![{}]({})\n\n", img_ref.alt, img_ref.path));
            }
        }
    }

    result
}

/// Parse a page marker to extract the page number.
///
/// Supports two formats:
/// - `<!-- Page N -->` (HTML comment style)
/// - `## Page N` (heading style, used by MarkdownRenderer)
///
/// Returns the 1-based page number, or None if not a page marker.
#[cfg(feature = "pdfium")]
fn parse_page_marker(line: &str) -> Option<usize> {
    let trimmed = line.trim();

    // Format 1: <!-- Page N -->
    if trimmed.starts_with("<!-- Page ") && trimmed.ends_with(" -->") {
        let inner = &trimmed[10..trimmed.len() - 4];
        return inner.trim().parse::<usize>().ok();
    }

    // Format 2: ## Page N (heading style from MarkdownRenderer)
    if let Some(rest) = trimmed.strip_prefix("## Page ") {
        return rest.trim().parse::<usize>().ok();
    }

    None
}
