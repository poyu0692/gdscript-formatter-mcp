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

### 方法1: インストールスクリプト使用（推奨）

自動的にプリビルドバイナリをダウンロードして配置します。

**Linux / macOS:**

```bash
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp
scripts/install.sh install
```

インストール先: `~/.local/bin/gdscript-formatter-mcp`

**Windows:**

```powershell
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp
.\scripts\install.ps1 install
```

インストール先: `%LOCALAPPDATA%\bin\gdscript-formatter-mcp.exe`

**動作確認:**

```bash
# Linux/macOS
scripts/install.sh doctor

# Windows
.\scripts\install.ps1 doctor
```

### 方法2: 手動インストール

[GitHub Releases](https://github.com/poyu0692/gdscript-formatter-mcp/releases) から環境に合ったバイナリをダウンロードします。

**Linux / macOS:**

```bash
# ダウンロード（例: Linux x86_64）
curl -LO https://github.com/poyu0692/gdscript-formatter-mcp/releases/latest/download/gdscript-formatter-mcp-x86_64-unknown-linux-gnu.tar.gz

# 解凍
tar xzf gdscript-formatter-mcp-x86_64-unknown-linux-gnu.tar.gz

# 任意の場所に配置（例: ~/.local/bin）
mkdir -p ~/.local/bin
mv gdscript-formatter-mcp ~/.local/bin/
chmod +x ~/.local/bin/gdscript-formatter-mcp
```

**Windows:**

1. [Releases ページ](https://github.com/poyu0692/gdscript-formatter-mcp/releases) から `gdscript-formatter-mcp-x86_64-pc-windows-msvc.zip` をダウンロード
2. 解凍して `gdscript-formatter-mcp.exe` を任意の場所に配置
3. MCP クライアント設定でそのパスを指定

### 方法3: ソースからビルド（開発者向け）

Rust ツールチェインが必要です。

```bash
git clone https://github.com/poyu0692/gdscript-formatter-mcp.git
cd gdscript-formatter-mcp
cargo build --release

# オプション: インストールスクリプト経由でインストール
scripts/install.sh install --from-source
```

ビルド成果物: `./target/release/gdscript-formatter-mcp`

### インストールスクリプトの追加オプション

```bash
# 特定バージョンをインストール
scripts/install.sh install --version v0.1.0

# バージョン一覧とステータス確認
scripts/install.sh status

# バージョン切り替え
scripts/install.sh link 0.1.0

# アンインストール
scripts/install.sh uninstall 0.1.0
scripts/install.sh uninstall --all  # 全バージョン削除
```

## MCP クライアント設定

Claude Desktop などの MCP クライアントで使用する場合、設定ファイルに以下を追加します。

### インストールスクリプトを使用した場合

**Linux / macOS** (`~/.config/claude-desktop/config.json`):

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "/home/YOUR_USERNAME/.local/bin/gdscript-formatter-mcp"
    }
  }
}
```

`YOUR_USERNAME` を実際のユーザー名に置き換えてください。または `~/.local/bin/gdscript-formatter-mcp` のように `~` を使える場合もあります。

**Windows** (`%APPDATA%\Claude\config.json`):

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "C:\\Users\\YOUR_USERNAME\\AppData\\Local\\bin\\gdscript-formatter-mcp.exe"
    }
  }
}
```

`YOUR_USERNAME` を実際のユーザー名に置き換えてください。

### 手動インストール or ソースビルドの場合

バイナリを配置した実際のパスを指定してください。

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "/path/to/gdscript-formatter-mcp"
    }
  }
}
```

### Claude Code (CLI)

**全プラットフォーム** (`~/.claude/config.json`):

```json
{
  "mcpServers": {
    "gdscript-formatter": {
      "command": "~/.local/bin/gdscript-formatter-mcp"
    }
  }
}
```

Windows の場合は `%USERPROFILE%\.local\bin\gdscript-formatter-mcp.exe` などのパスを指定してください。

**設定後は MCP クライアントを再起動してください。**

## プロジェクトでの使い方

### AGENTS.md への記載例

Godot プロジェクトの `AGENTS.md` に以下のような指示を追加すると、AI がコード編集後に自動的にフォーマット＆Lint を実行してくれます。

```markdown
# GDScript コーディング規約

## フォーマットと Lint

コードを編集・追加したら、**必ず以下を実行してください**:

### 1. フォーマット実行

`gdscript_format` ツールを使用してコードを整形します。

**単一ファイルの場合:**
- ツール: `gdscript_format`
- 引数: `{"files": ["path/to/edited.gd"]}`

**ディレクトリ全体の場合:**
- ツール: `gdscript_format`
- 引数: `{"dir": "addons/your_addon", "include": ["**/*.gd"]}`

**チェックのみ（変更しない）:**
- 引数に `"check": true` を追加

### 2. Lint 実行

`gdscript_lint` ツールでコード品質をチェックします。

**単一ファイルの場合:**
- ツール: `gdscript_lint`
- 引数: `{"files": ["path/to/edited.gd"]}`

**ディレクトリ全体の場合:**
- ツール: `gdscript_lint`
- 引数: `{"dir": "addons/your_addon", "include": ["**/*.gd"]}`

### 3. 問題があれば修正して再実行

Lint でエラーや警告が出た場合は、コードを修正してから再度フォーマット＆Lint を実行してください。

## 典型的なワークフロー

1. GDScript ファイルを編集
2. `gdscript_format` で整形（`check: false`）
3. `gdscript_lint` でチェック
4. 問題があれば修正して 2-3 を繰り返す
5. コミット前に全体を再チェック（`check: true`）
```

### プロジェクト全体の定期チェック

コミット前やプルリクエスト作成前に、プロジェクト全体をチェックする習慣をつけると良いでしょう。

```bash
# MCP クライアント経由で以下を実行
gdscript_format:
  dir: "."
  include: ["**/*.gd"]
  exclude: ["addons/third_party/**"]
  check: true

gdscript_lint:
  dir: "."
  include: ["**/*.gd"]
  exclude: ["addons/third_party/**"]
```

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

## 使用例

MCP ツールとしての入出力例です。AI がこれらのツールを呼び出すと、以下のような形式で結果が返されます。

**注意**: 返却内容はコンテキスト節約のため最小化されています。詳細ログが必要な場合は `gdscript_lint` で `include_raw_output=true` を指定してください。

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
