# t2s_client

A text-to-speech (TTS) client using the AivisSpeech API.
Converts text files and Markdown files to speech and outputs them as WAV files.

[日本語ドキュメント](docs/README-JP.md)

## Features

- Read txt/md files and convert to speech
- Combine multiple files into a single WAV output
- Automatically skip Markdown headers (`#`) and code blocks (```)
- Automatically split long text into chunks for processing
- Optional realtime playback
- Speaker/style selection

## Requirements

- [AivisSpeech](https://aivis-project.com/) running locally
- By default, expects the API at `http://127.0.0.1:10101`

## Installation

```bash
cargo build --release
```

## Usage

### Basic usage

```bash
t2s_client -i <input file/folder> -o <output WAV file>
```

### Examples

```bash
# Convert a single file
t2s_client -i document.md -o output.wav

# Convert all txt/md files in a folder
t2s_client -i ./documents/ -o combined.wav

# Specify a speaker
t2s_client -i document.md -o output.wav -s 1995743776

# Enable realtime playback during conversion
t2s_client -i document.md -o output.wav -r

# Change speech speed (1.5x)
t2s_client -i document.md -o output.wav --speed 1.5
```

### List available speakers

```bash
t2s_client --list-speakers
```

## Options

| Option | Short | Description | Default |
| ------ | ----- | ----------- | ------- |
| `--input` | `-i` | Input file or folder | (required) |
| `--output` | `-o` | Output WAV file path | (required) |
| `--speaker` | `-s` | Speaker ID | 888753760 |
| `--speed` | - | Speech speed (1.0 = normal) | 1.0 |
| `--realtime` | `-r` | Enable realtime playback | false |
| `--api-url` | - | AivisSpeech API URL | `http://127.0.0.1:10101` |
| `--list-speakers` | - | List available speakers | - |

## License

MIT
