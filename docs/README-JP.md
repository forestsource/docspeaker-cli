# docspeaker-cli

AivisSpeech API を使用したテキスト読み上げ (TTS) クライアント。
テキストファイルや Markdown ファイルを音声に変換し、WAV ファイルとして出力します。

## 機能

- txt/md ファイルを読み込み、音声に変換
- 複数ファイルを結合して1つの WAV ファイルに出力
- Markdown のヘッダー (`#`) やコードブロック (```) を自動スキップ
- 長いテキストを自動的に分割して処理
- リアルタイム再生オプション
- 話者・スタイルの選択

## 必要条件

- [AivisSpeech](https://aivis-project.com/) がローカルで起動していること
- デフォルトでは `http://127.0.0.1:10101` で API が動作している前提

## インストール

```bash
cargo build --release
```

## 使い方

### 基本的な使い方

```bash
docspeaker-cli -i <入力ファイル/フォルダ> -o <出力WAVファイル>
```

### 例

```bash
# 単一ファイルを変換
docspeaker-cli -i document.md -o output.wav

# フォルダ内の全 txt/md ファイルを変換
docspeaker-cli -i ./documents/ -o combined.wav

# 話者を指定
docspeaker-cli -i document.md -o output.wav -s 1995743776

# リアルタイム再生しながら変換
docspeaker-cli -i document.md -o output.wav -r

# 読み上げ速度を変更 (1.5倍速)
docspeaker-cli -i document.md -o output.wav --speed 1.5
```

### 利用可能な話者を確認

```bash
docspeaker-cli --list-speakers
```

## オプション

| オプション | 短縮 | 説明 | デフォルト |
| ----------- | ---- | ---- | ----------- |
| `--input` | `-i` | 入力ファイルまたはフォルダ | (必須) |
| `--output` | `-o` | 出力 WAV ファイルパス | (必須) |
| `--speaker` | `-s` | 話者 ID | 888753760 |
| `--speed` | - | 読み上げ速度 (1.0 = 通常) | 1.0 |
| `--realtime` | `-r` | リアルタイム再生を有効化 | false |
| `--api-url` | - | AivisSpeech API の URL | `http://127.0.0.1:10101` |
| `--list-speakers` | - | 利用可能な話者一覧を表示 | - |

## 設定ファイル

`settings.json` ファイルでデフォルト値を設定できます。以下の場所が優先順位順にチェックされます：

1. 実行ファイルと同じディレクトリ (`settings.json`)
2. `$HOME/.config/docspeaker-cli/settings.json`

コマンドライン引数は常に設定ファイルの値より優先されます。

### settings.json の例

```json
{
  "speaker": 1995743776,
  "api_url": "http://127.0.0.1:10101",
  "speed": 1.0,
  "realtime": false
}
```

## ライセンス

MIT
