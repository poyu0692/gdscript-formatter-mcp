# gdscript-formatter-mcp

GDQuest の `GDScript-formatter` 最新リリースを自動取得して、MCP ツールとして `format` / `lint` を提供する Rust 製サーバーです。

## 背景（なぜ作ったか）

毎回 `AGENTS.md` に formatter / linter の実行方法や引数を説明するのが手間だったため、MCP ツールとして標準化しました。
このサーバーを入れておけば、AI が `gdscript_format` / `gdscript_lint` を直接呼べるので、プロジェクトごとに運用ルールを何度も教え直す必要が減ります。

## これで解決できること

- AI に「どのコマンドをどう叩くか」を毎回説明しなくてよくなる
- lint 結果を `structuredContent` で受け取れるため、AI が `file/line/rule` ベースで修正提案しやすい
- ファイル単位で継続処理するため、1ファイル失敗で全体停止しにくい
- `dir + include/exclude` で対象ファイルの絞り込みをツール引数で完結できる
- 最新 formatter の取得を自動化しつつ、キャッシュで安定運用できる

## 何をするか

- GitHub API (`releases/latest`) から最新バージョンを取得
- 実行環境の OS / CPU に合った zip アセットを自動ダウンロード
- ローカルキャッシュに展開して再利用
- MCP ツールを提供
  - `gdscript_format`
  - `gdscript_lint`

2026-02-14 時点で GitHub API の最新タグは `0.18.2` でした（実装はタグを固定せず毎回 latest を参照）。

## 元プロジェクト（謝辞）

この MCP サーバーは、GDQuest の `GDScript-formatter` を利用しています。素晴らしいプロジェクトに感謝します。

- Repository: https://github.com/GDQuest/GDScript-formatter
- Latest release: https://github.com/GDQuest/GDScript-formatter/releases/latest

## Fork / 改造 / Contribution 大歓迎

このリポジトリは、以下のような関わり方をすべて歓迎しています。

- fork して自由に改造する
- Issue / Pull Request で contribution する
- アイデアだけ持っていって自作する
- 厳しめのフィードバックや酷評を送る

小さな改善でも、大きな方向転換でも歓迎です。使いやすい形で気軽に育ててください。

## インストール

### プリビルドバイナリ（推奨）

各プラットフォーム向けのプリビルドバイナリを [GitHub Releases](https://github.com/poyu0692/gdscript-formatter-mcp/releases) からダウンロードできます。

#### Linux / macOS

```bash
# リポジトリをクローン（インストールスクリプトを使う場合）
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp

# 最新版を自動インストール
scripts/install.sh install

# 動作確認
scripts/install.sh doctor
```

インストール先：
- バイナリ: `~/.local/share/mcp/gdscript-formatter-mcp/<version>/`
- シンボリックリンク: `~/.local/bin/gdscript-formatter-mcp`

#### Windows

```powershell
# リポジトリをクローン（インストールスクリプトを使う場合）
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp

# 最新版を自動インストール
.\scripts\install.ps1 install

# 動作確認
.\scripts\install.ps1 doctor
```

インストール先：
- バイナリ: `%LOCALAPPDATA%\mcp\gdscript-formatter-mcp\<version>\`
- シンボリックリンク: `%LOCALAPPDATA%\bin\gdscript-formatter-mcp.exe`

#### 手動インストール

[Releases ページ](https://github.com/poyu0692/gdscript-formatter-mcp/releases) から環境に合ったバイナリをダウンロードして、任意の場所に配置してください。

### ソースからビルド

Rust ツールチェインが必要です。

```bash
# リポジトリをクローン
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp

# リリースビルド
cargo build --release

# インストール（オプション）
scripts/install.sh install --from-source
```

実行バイナリ: `./target/release/gdscript-formatter-mcp`

### インストーラのサブコマンド

```bash
# 特定バージョンをインストール
scripts/install.sh install --version v0.1.0

# インストール状態確認
scripts/install.sh status

# バージョン切り替え
scripts/install.sh link 0.1.0

# アンインストール
scripts/install.sh uninstall 0.1.0
scripts/install.sh uninstall --all  # 全バージョン削除

# MCP 疎通確認
scripts/install.sh doctor
```

## MCP クライアント設定

Claude Desktop や他の MCP クライアントで使用する場合、設定ファイルに追加します。

**Linux / macOS** (`~/.config/claude-desktop/config.json` など):

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "/home/YOUR_USERNAME/.local/bin/gdscript-formatter-mcp"
    }
  }
}
```

**Windows** (`%APPDATA%\Claude\config.json` など):

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "C:\\Users\\YOUR_USERNAME\\AppData\\Local\\bin\\gdscript-formatter-mcp.exe"
    }
  }
}
```

設定後、MCP クライアントを再起動してください。

## 提供ツール

### `gdscript_format`

主な引数:

- `files` (string[]): 対象ファイル配列
- `dir` (string): 走査対象ディレクトリ
- `include` (string[]): `dir` からの相対glob（既定: `["**/*.gd"]`）
- `exclude` (string[]): `dir` からの相対glob除外
- `check` (bool): 変更せず整形状態のみ確認
- `stdout` (bool): ファイル更新せず標準出力へ出力
- `use_spaces` (bool)
- `indent_size` (int, >=1)
- `reorder_code` (bool)
- `safe` (bool)

`files` と `dir` は併用可能です（重複は自動除外）。

返却は最小化されており、`structuredContent` は以下です。

注: `gdscript_format` の `structuredContent` は軽量化のため破壊的に変更されています（旧 `exit_code` や `successful_files` 等は返しません）。

- 成功時:
  - `ok` (bool)
  - `processed_count` (int): 処理したファイル数
- 失敗時:
  - `ok` (bool)
  - `processed_count` (int): 処理したファイル数
  - `failed_count` (int)
  - `failures_truncated` (bool)
  - `failures` (array)
  - `file`, `reason`

### `gdscript_lint`

主な引数:

- `files` (string[]): 対象ファイル配列
- `dir` (string): 走査対象ディレクトリ
- `include` (string[]): `dir` からの相対glob（既定: `["**/*.gd"]`）
- `exclude` (string[]): `dir` からの相対glob除外
- `disable_rules` (string): カンマ区切り
- `max_line_length` (int, >=1)
- `list_rules` (bool)
- `pretty` (bool)
- `include_raw_output` (bool): `structuredContent` に `raw_stdout/raw_stderr` を含める
- `max_diagnostics` (int, 既定 `500`): 返す diagnostics 件数上限

`list_rules=true` 以外では、`files` または `dir` のいずれかで対象を指定します。

返却は `content` のテキストに加えて、`structuredContent` も含みます。

- `ok` (bool)
- `exit_code` (int)
- `total_diagnostics` (int)
- `error_count` (int)
- `warning_count` (int)
- `diagnostics_truncated` (bool)
- `diagnostics` (array)
  - `file`, `line`, `column`, `rule`, `severity`, `message`
- `raw_stdout` / `raw_stderr` は `include_raw_output=true` の時のみ返却

## AI 入出力例（tools/call）

デフォルトはコンテキスト節約のため軽量返却です。詳細ログが必要な場合だけ `gdscript_lint` で `include_raw_output=true` を指定してください（`max_diagnostics` で件数制御できます）。

### `gdscript_format` の例

入力:

```json
{
  "name": "gdscript_format",
  "arguments": {
    "dir": "addons",
    "include": ["**/*.gd"],
    "exclude": ["**/vendor/**"],
    "check": true
  }
}
```

成功時の出力例（抜粋）:

```json
{
  "isError": false,
  "content": [
    {
      "type": "text",
      "text": "Format ok."
    }
  ],
  "structuredContent": {
    "ok": true,
    "processed_count": 15
  }
}
```

失敗時の出力例（抜粋）:

```json
{
  "isError": true,
  "content": [
    {
      "type": "text",
      "text": "Format failed. failed_count=1."
    }
  ],
  "structuredContent": {
    "ok": false,
    "processed_count": 1,
    "failed_count": 1,
    "failures_truncated": false,
    "failures": [
      {
        "file": "addons/example/bad.gd",
        "reason": "Topiary formatting failed: Trying to close an unopened indentation block"
      }
    ]
  }
}
```

### `gdscript_lint` の例

入力:

```json
{
  "name": "gdscript_lint",
  "arguments": {
    "dir": "addons",
    "include": ["**/*.gd"],
    "pretty": false
  }
}
```

成功時の出力例（抜粋）:

```json
{
  "isError": false,
  "content": [
    {
      "type": "text",
      "text": "Lint completed successfully. diagnostics: total=0, errors=0, warnings=0"
    }
  ],
  "structuredContent": {
    "ok": true,
    "exit_code": 0,
    "total_diagnostics": 0,
    "error_count": 0,
    "warning_count": 0,
    "max_diagnostics": 500,
    "diagnostics_truncated": false,
    "diagnostics": []
  }
}
```

失敗時の出力例（抜粋）:

```json
{
  "isError": true,
  "content": [
    {
      "type": "text",
      "text": "Lint failed. ..."
    }
  ],
  "structuredContent": {
    "ok": false,
    "exit_code": 1,
    "diagnostics": [
      {
        "file": "addons/player/player.gd",
        "line": 12,
        "column": null,
        "rule": "max-line-length",
        "severity": "warning",
        "message": "Line exceeds max length"
      }
    ],
    "raw_stdout": "...",
    "raw_stderr": ""
  }
}
```

## 環境変数

- `GDSCRIPT_FORMATTER_PATH`
  - 既存の `gdscript-formatter` 実行ファイルを固定利用したい時に指定
- `GDSCRIPT_FORMATTER_MCP_CACHE_DIR`
  - ダウンロードキャッシュ先を明示したい時に指定

## 補足

- デフォルトのキャッシュ先は以下の順で解決します。
  1. `GDSCRIPT_FORMATTER_MCP_CACHE_DIR`
  2. `XDG_CACHE_HOME/gdscript-formatter-mcp` または `~/.cache/gdscript-formatter-mcp`
  3. カレント配下 `.gdscript-formatter-mcp-cache`
  4. 一時ディレクトリ配下
