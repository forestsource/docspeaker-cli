use anyhow::{Context, Result};
use clap::Parser;
use hound::{SampleFormat, WavSpec, WavWriter};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use walkdir::WalkDir;

const DEFAULT_API_URL: &str = "http://127.0.0.1:10101";
const MAX_CHARS_PER_REQUEST: usize = 150;

#[derive(Parser, Debug)]
#[command(name = "docspeaker-cli")]
#[command(about = "AivisSpeech client for text-to-speech conversion")]
struct Args {
    /// Input folder containing txt/md files
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output WAV file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Speaker ID (use /speakers endpoint to find available speakers)
    #[arg(short, long, default_value = "888753760")]
    speaker: i64,

    /// AivisSpeech API URL
    #[arg(long, default_value = DEFAULT_API_URL)]
    api_url: String,

    /// Speed scale (1.0 = normal)
    #[arg(long, default_value = "1.0")]
    speed: f32,

    /// Enable realtime playback
    #[arg(short, long)]
    realtime: bool,

    /// List available speakers and exit
    #[arg(long)]
    list_speakers: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AudioQuery {
    accent_phrases: Vec<serde_json::Value>,
    #[serde(rename = "speedScale")]
    speed_scale: f32,
    #[serde(rename = "pitchScale")]
    pitch_scale: f32,
    #[serde(rename = "intonationScale")]
    intonation_scale: f32,
    #[serde(rename = "volumeScale")]
    volume_scale: f32,
    #[serde(rename = "prePhonemeLength")]
    pre_phoneme_length: f32,
    #[serde(rename = "postPhonemeLength")]
    post_phoneme_length: f32,
    #[serde(rename = "outputSamplingRate")]
    output_sampling_rate: i32,
    #[serde(rename = "outputStereo")]
    output_stereo: bool,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Speaker {
    name: String,
    styles: Vec<Style>,
}

#[derive(Debug, Deserialize)]
struct Style {
    name: String,
    id: i64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle --list-speakers
    if args.list_speakers {
        let client = reqwest::blocking::Client::new();
        let speakers = list_speakers(&client, &args.api_url)?;
        println!("Available speakers:");
        println!("===================");
        for speaker in speakers {
            println!("\n{}", speaker.name);
            for style in speaker.styles {
                println!("  - {} (ID: {})", style.name, style.id);
            }
        }
        return Ok(());
    }

    // Validate required arguments
    let input = args.input.context("--input is required")?;
    let output = args.output.context("--output is required")?;

    println!("AivisSpeech Text-to-Speech Client");
    println!("==================================");
    println!("Input folder: {:?}", input);
    println!("Output file: {:?}", output);
    println!("Speaker ID: {}", args.speaker);
    println!("API URL: {}", args.api_url);
    println!();

    // Collect all txt and md files
    let files = collect_text_files(&input)?;
    if files.is_empty() {
        println!("No txt or md files found in the specified folder.");
        return Ok(());
    }

    println!("Found {} file(s) to process:", files.len());
    for f in &files {
        println!("  - {:?}", f);
    }
    println!();

    // Read and combine all text
    let mut all_text = String::new();
    for file in &files {
        let content =
            fs::read_to_string(file).with_context(|| format!("Failed to read file: {:?}", file))?;
        if !all_text.is_empty() {
            all_text.push_str("\n\n");
        }
        all_text.push_str(&content);
    }

    // Parse and split text into chunks
    let chunks = split_text_into_chunks(&all_text, MAX_CHARS_PER_REQUEST);
    println!("Split text into {} chunk(s)", chunks.len());
    println!();

    // Setup audio output
    let (_stream, stream_handle) =
        OutputStream::try_default().context("Failed to get audio output device")?;
    let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

    // Process each chunk and collect WAV data
    let client = reqwest::blocking::Client::new();
    let mut all_samples: Vec<i16> = Vec::new();
    let mut sample_rate: u32 = 44100;

    for (i, chunk) in chunks.iter().enumerate() {
        let preview: String = chunk.chars().take(30).collect();
        let ellipsis = if chunk.chars().count() > 30 {
            "..."
        } else {
            ""
        };
        println!(
            "[{}/{}] Processing: {}{}",
            i + 1,
            chunks.len(),
            preview,
            ellipsis
        );

        // Step 1: Generate audio query
        let query = generate_audio_query(&client, &args.api_url, chunk, args.speaker, args.speed)?;

        // Step 2: Synthesize audio
        let wav_data = synthesize_audio(&client, &args.api_url, &query, args.speaker)?;

        // Read WAV data to get samples
        let cursor = Cursor::new(&wav_data);
        let reader = hound::WavReader::new(cursor).context("Failed to parse WAV data")?;

        let spec = reader.spec();
        sample_rate = spec.sample_rate;

        let samples: Vec<i16> = reader
            .into_samples::<i16>()
            .filter_map(|s| s.ok())
            .collect();

        all_samples.extend(&samples);

        // Play audio in realtime (if enabled)
        if args.realtime {
            let cursor = Cursor::new(wav_data);
            if let Ok(source) = Decoder::new(cursor) {
                sink.append(source);
            }
        }
    }

    // Wait for playback to finish
    if args.realtime {
        println!();
        println!("Playing audio... (waiting for completion)");
        sink.sleep_until_end();
    }

    // Write combined WAV file
    println!();
    println!("Writing output file: {:?}", output);

    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer =
        WavWriter::create(&output, spec).context("Failed to create output WAV file")?;

    for sample in all_samples {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    println!("Done! Output saved to {:?}", output);

    Ok(())
}

fn collect_text_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "txt" || ext == "md" {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

fn split_text_into_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();

    // Clean the text
    let text = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#')) // Skip markdown headers
        .filter(|line| !line.starts_with("```")) // Skip code blocks
        .collect::<Vec<_>>()
        .join(" ");

    // Split by sentence-ending punctuation
    let mut current_chunk = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        current_chunk.push(c);

        // Check for sentence endings
        let is_sentence_end = matches!(c, '。' | '！' | '？' | '.' | '!' | '?')
            && chars.peek().map_or(true, |&next| {
                next == ' '
                    || next == '　'
                    || next.is_ascii_whitespace()
                    || !next.is_ascii_punctuation()
            });

        if is_sentence_end || current_chunk.chars().count() >= max_chars {
            let trimmed = current_chunk.trim().to_string();
            if !trimmed.is_empty() {
                chunks.push(trimmed);
            }
            current_chunk = String::new();
        }
    }

    // Don't forget the last chunk
    let trimmed = current_chunk.trim().to_string();
    if !trimmed.is_empty() {
        chunks.push(trimmed);
    }

    // Merge very short chunks with the previous one
    let mut merged = Vec::new();
    for chunk in chunks {
        if let Some(last) = merged.last_mut() {
            let last_str: &mut String = last;
            if last_str.chars().count() + chunk.chars().count() < max_chars / 2 {
                last_str.push_str(&chunk);
                continue;
            }
        }
        merged.push(chunk);
    }

    merged
}

fn generate_audio_query(
    client: &reqwest::blocking::Client,
    api_url: &str,
    text: &str,
    speaker: i64,
    speed: f32,
) -> Result<AudioQuery> {
    let url = format!(
        "{}/audio_query?text={}&speaker={}",
        api_url,
        urlencoding::encode(text),
        speaker
    );

    let response = client
        .post(&url)
        .header("Accept", "application/json")
        .send()
        .context("Failed to send audio_query request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!("audio_query failed with status {}: {}", status, body);
    }

    let mut query: AudioQuery = response
        .json()
        .context("Failed to parse audio_query response")?;

    query.speed_scale = speed;

    Ok(query)
}

fn synthesize_audio(
    client: &reqwest::blocking::Client,
    api_url: &str,
    query: &AudioQuery,
    speaker: i64,
) -> Result<Vec<u8>> {
    let url = format!("{}/synthesis?speaker={}", api_url, speaker);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(query)
        .send()
        .context("Failed to send synthesis request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        anyhow::bail!("synthesis failed with status {}: {}", status, body);
    }

    let wav_data = response.bytes().context("Failed to get WAV data")?.to_vec();

    Ok(wav_data)
}

fn list_speakers(client: &reqwest::blocking::Client, api_url: &str) -> Result<Vec<Speaker>> {
    let url = format!("{}/speakers", api_url);

    let response = client.get(&url).send().context("Failed to get speakers")?;

    let speakers: Vec<Speaker> = response
        .json()
        .context("Failed to parse speakers response")?;

    Ok(speakers)
}
